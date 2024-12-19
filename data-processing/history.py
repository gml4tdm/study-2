import collections
import contextlib
import functools
import hashlib
import logging
import os
import json
import pathlib
import re
import statistics
import subprocess
import sys

import alive_progress
import pydriller
import tap


def persistent_cache(function):
    @functools.wraps(function)
    def wrapper(*args, **kwargs):
        if not os.path.exists('cache'):
            os.mkdir('cache')
        key = (function.__name__,)
        key += args
        key += tuple(sorted(kwargs.items()))
        key_hash = hashlib.sha256(str(key).encode()).hexdigest()
        cache_file = os.path.join('cache', f'{function.__name__}_{key_hash}.json')
        if os.path.exists(cache_file):
            with open(cache_file, 'r') as f:
                return json.load(f)
        result = function(*args, **kwargs)
        with open(cache_file, 'w') as f:
            json.dump(result, f)
        return result
    return wrapper


class Config(tap.Tap):
    repo: str
    output_path: pathlib.Path
    commit_tags = None
    min_version: str | None = None
    max_version: str | None = None
    version_pattern: str | None = None


class GitCmdClient:

    def __init__(self, repo: str):
        self.repo = repo

    def _call(self, factory):
        with contextlib.chdir(self.repo):
            p = factory()
            out = list(iter(p.stdout.readline, b''))
            return b''.join(out).decode().strip()

    def call(self, cmd: list[str]) -> str:
        return self._call(lambda: subprocess.Popen(cmd, stdout=subprocess.PIPE))

    def call_shell(self, cmd: str) -> str:
        return self._call(lambda: subprocess.Popen(cmd, stdout=subprocess.PIPE, shell=True))


def count_commits(p: str):
    client = GitCmdClient(p)
    command = ['mergestat', "SELECT COUNT(*) FROM commits",  '-f', 'single']
    return int(client.call(command))


def try_int(x):
    try:
        return int(x)
    except ValueError:
        return x


@persistent_cache
def get_commits_for_versions(p: str, version_pattern: str | None):
    # Get minor version tags
    client = GitCmdClient(p)
    all_tags = [
        line.split()[1]
        for line in client.call_shell('git show-ref --tags').splitlines(keepends=False)
    ]
    if version_pattern is None:
        version_pattern = r'\d+\.\d+|\d+\.\d+\.0'
    pattern = re.compile(rf'^refs/tags/[^0-9]*(?P<version>{version_pattern})$')
    minor_release_tags = [
        (tag, m.group('version'))
        for tag in all_tags
        if (m := pattern.match(tag)) is not None
    ]

    print(minor_release_tags)

    # Sort by major version
    minor_by_major = collections.defaultdict(list)
    for tag, version in minor_release_tags:
        major = version.split('.')[0]
        minor_by_major[major].append((tag, version))
    for v in minor_by_major.values():
        v.sort(key=lambda x: tuple(map(try_int, x[1].split('.'))))

    # result = {}
    # with alive_progress.alive_bar(len(minor_release_tags) - len(minor_by_major)) as bar:
    #     for major, tags in minor_by_major.items():
    #         result[major] = {}
    #         for (tag_old, v_old), (tag_new, version) in zip(tags[:-1], tags[1:], strict=True):
    #             log = client.call_shell(f'git log --pretty=format:"%H" {tag_old}..{tag_new}')
    #             minor = version.split('.')[1]
    #             result[major][minor] = {
    #                 'old-version': v_old,
    #                 'new-version': version,
    #                 'tag-old': tag_old,
    #                 'tag-new': tag_new,
    #                 'commits': log.splitlines(keepends=False)
    #             }
    #             bar()

    # Get commits between minor versions, accounting for the fact
    # that we may diverge from the main branch for the last
    # number of consecutive minor versions
    main_branch = client.call_shell('git branch --show-current')
    result = {}
    with alive_progress.alive_bar(len(minor_release_tags) - len(minor_by_major)) as bar:
        for major, tags in minor_by_major.items():
            result[major] = {
            }
            last_tag = tags[-1][0]
            last_divergence_point = client.call_shell(f'git merge-base {main_branch} {last_tag}')
            for (tag_old, v_old), (tag_new, version) in zip(tags[:-1], tags[1:], strict=True):
                minor = version.split('.')[1]
                version_divergence_point = client.call_shell(f'git merge-base {tag_old} {tag_new}')
                divergence_point_with_main = client.call_shell(f'git merge-base {tag_old} {main_branch}')
                if version_divergence_point == divergence_point_with_main:
                    new_version_divergence_point = client.call_shell(f'git merge-base {tag_new} {main_branch}')
                    log = client.call_shell(f'git log --pretty=format:"%H" --no-decorate {version_divergence_point}..{new_version_divergence_point}')
                else:
                    if divergence_point_with_main != last_divergence_point:
                        raise ValueError(f'Version {version} diverged from a prior divergent version which is not in the final tail of minor versions')
                    divergence_point_stop = client.call_shell(f'git merge-base {tag_new} {last_tag}')
                    log = client.call_shell(f'git log --pretty=format:"%H" --no-decorate {version_divergence_point}..{divergence_point_stop}')
                result[major][minor] = {
                    'old-version': v_old,
                    'new-version': version,
                    'commits': log.splitlines(keepends=False)
                }
                bar()

    return result




def get_package(source: str | None) -> str | None:
    if source is None:
        return None
    pattern = re.compile(r'\s*package (?P<name>[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s*;')
    m = pattern.search(source)
    if m is None:
        return None
    return m.group('name')


@persistent_cache
def mine_change_information_for_version(repo_path, major, minor, tag_old, tag_new, v_min, v_max, commits):
    if v_min is not None and (int(major), int(minor)) < tuple(map(int, v_min.split('.'))):
        return {}
    if v_max is not None and (int(major), int(minor)) > tuple(map(int, v_max.split('.'))):
        return {}
    repo = pydriller.Repository(repo_path, only_commits=commits)
    result = {}
    with alive_progress.alive_bar(len(commits)) as bar:
        print(f'{major}.{minor} (commit range: {commits[-1]}..{commits[0]}; tag range: {tag_old}..{tag_new})')
        for seq, commit in enumerate(repo.traverse_commits()):
            result[commit.hash] = {
                'seq': seq,
                'author_date_ts': commit.author_date.timestamp(),
                'committer_date_ts': commit.author_date.timestamp(),
                'files': [
                    {
                        'name': file.filename,
                        'name_old': file.old_path,
                        'name_new': file.new_path,
                        'package_old': get_package(file.source_code_before),
                        'package_new': get_package(file.source_code),
                        'action': file.change_type.name,
                        'methods_before': [meth.name for meth in file.methods_before],
                        'methods_after': [meth.name for meth in file.methods],
                        'methods_changed': [meth.name for meth in file.changed_methods]
                    }
                    for file in commit.modified_files
                ]
            }
            bar()
        analysed = set(result)
        required = set(commits)
        if required != analysed:
            raise ValueError(f'Commits {required - analysed} are missing from the analysis')

    return result


def main(config: Config):
    #logging.getLogger().setLevel(logging.DEBUG)
    logger = logging.getLogger(__name__)
    formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    handler = logging.StreamHandler(sys.stdout)
    handler.setFormatter(formatter)
    logger.addHandler(handler)

    #logging.getLogger('pydriller').addHandler(handler)

    handler = logging.FileHandler('history.log', mode='w')
    handler.setFormatter(formatter)
    logger.addHandler(handler)

    logger.setLevel(logging.INFO)

    commits = get_commits_for_versions(config.repo, config.version_pattern)

    to_delete_major = []
    for major, minors in commits.items():
        to_delete_minor = []
        for minor, data in minors.items():
            data['commit_change_data'] = mine_change_information_for_version(config.repo,
                                                                             major,
                                                                             minor,
                                                                             data.pop('tag-old', None),
                                                                             data.pop('tag-new', None),
                                                                             config.min_version,
                                                                             config.max_version,
                                                                             data['commits'])
            if not data['commit_change_data']:
                to_delete_minor.append(minor)
        for minor in to_delete_minor:
            print(f'Deleting {minor} from {major}')
            del minors[minor]
        if not minors:
            to_delete_major.append(major)
    for major in to_delete_major:
        print(f'Deleting {major}')
        del commits[major]


    with open(config.output_path, 'w') as fp:
        json.dump(commits, fp, indent=2)



if __name__ == "__main__":
    main(Config().parse_args())
