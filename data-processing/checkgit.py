import contextlib
import os
import subprocess

import alive_progress


def _call(cmd: list[str]):
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE)
    p.wait()
    out = p.stdout.read().decode()
    return out.strip()


def check_git(repo: str):
    with contextlib.chdir(repo):
        out = _call(['git', 'show-ref', '--tags', '-d'])
        pairs = [
            tuple(map(str.strip, x.split())) for x in out.splitlines()
        ]
        failures = []
        with alive_progress.alive_bar(len(pairs)) as bar:
            for commit, tag in pairs:
                out = _call(['git', 'branch', '--contains', commit]).strip()
                if not out:
                    failures.append((commit, tag))
                bar()
    return len(pairs), failures


def check_git_jumps(repo: str, tags: list[str]):
    with contextlib.chdir(repo):
        for old, new in zip(tags, tags[1:]):
            out = _call(['git', 'rev-list', '--ancestry-path', '--remotes', f'{old}..{new}'])
            # if not out:
            #     out = _call(['git', 'rev-list', '--ancestry-path', f'{old}..{new}'])
            if not out:
                raise ValueError(f'Cannot resolve commits between {old} and {new}')
            hash_old = _call(['git', 'rev-list', '--remotes', f'tags/{old}'])
            hash_new = _call(['git', 'rev-list', '--remotes', f'tags/{new}'])
            history = out.splitlines()
            print(f'{old} -> {new}: {hash_old} -> {hash_new} -- {len(history)}')

def main():
    for p in ['ant', 'camel']:
        total, failures = check_git(os.path.expanduser(f'~/Desktop/repos/{p}'))
        for h, t in sorted(failures, key=lambda x: x[1]):
            print(f'{h} {t}')
        print(f'Apache {p.capitalize()}: {len(failures)} / {total} failed')



    # check_git_jumps(
    #     os.path.expanduser('~/Desktop/repos/camel'),
    #     [
    #         # 'rel/1.1',
    #         # 'rel/1.2',
    #         # 'rel/1.3',
    #         # 'rel/1.4',
    #         # 'rel/1.5',
    #         # 'rel/1.5.2',
    #         # 'rel/1.6.0',
    #         # 'rel/1.7.0',
    #         # 'rel/1.8.0',
    #         # 'rel/1.9.0',
    #         # 'rel/1.10.0',
    #         f'camel-2.{i}.0' for i in range(0, 18)
    #     ]
    # )

    # total, failures = check_git(os.path.expanduser('~/Desktop/repos/camel'))
    # print(failures)
    # print(f'Apache Camel: {len(failures)} / {total} failed')


if __name__ == "__main__":
    main()
