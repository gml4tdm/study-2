import os
import json
import subprocess

import alive_progress
import pydriller
import tap


class Config(tap.Tap):
    repo: str = os.path.expanduser('~/Desktop/ant')


def run_git_cmd(cmd: list[str], p: str):
    old = os.getcwd()
    os.chdir(p)
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE)
    p.wait()
    out = p.stdout.read().decode()
    os.chdir(old)
    return out

def count_commits(p: str):
    command = ['mergestat', "SELECT COUNT(*) FROM commits",  '-f', 'single']
    return int(run_git_cmd(command, p))


def get_tag_mapping(p: str):
    # Phase 1 -- Get tags
    command = ['git', 'show-ref', '--tags']
    out = run_git_cmd(command, p)

    # Phase 2 -- Map tags to commits
    # git rev-list -n 1 $TAG
    mapping = {}
    for line in out.splitlines():
        tag = line.split(' ', maxsplit=1)[1]
        commit_hash = run_git_cmd(
            ['git', 'rev-list', '-n', '1', tag],
            p
        )
        mapping.setdefault(commit_hash, []).append(tag)

    return mapping


def main(config: Config):
    repo = pydriller.Repository(config.repo)
    result = []
    tags = get_tag_mapping(config.repo)
    print(tags)
    with alive_progress.alive_bar(count_commits(config.repo)) as bar:
        for seq, commit in enumerate(repo.traverse_commits()):
            # We want the following information:
            # 1) author date in UTC
            # 2) commiter date in UTC
            # 3) sequence number of the commit
            # 4) Changed files (fully qualified paths ofc)
            # 5) if possible, changed classes/dependencies
            result.append({
                'seq': seq,
                'author_date_ts': commit.author_date.timestamp(),
                'committer_date_ts': commit.author_date.timestamp(),
                'tags': tags.pop(commit.hash, []),
                'files': [
                    {
                        'name': file.filename,
                        'name_old': file.old_path,
                        'name_new': file.new_path,
                        'action': file.change_type.name,
                        'methods_before': [meth.name for meth in file.methods_before],
                        'methods_after': [meth.name for meth in file.methods],
                        'methods_changed': [meth.name for meth in file.changed_methods]
                    }
                    for file in commit.modified_files
                ]
            })
            bar()

    with open('history.json', 'w') as fp:
        json.dump(result, fp)

    if tags:
        print(f'There were left-over tags: {tags}')


if __name__ == "__main__":
    main(Config().parse_args())

