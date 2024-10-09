import os.path
import urllib.request
import zipfile

REPO_LINK = 'https://github.com/tommantonela/ASPredictor/raw/refs/heads/master/reproducibility-kit'
ARCHIVES = (
    'apache-ant.zip',
    'apache-derby.zip'
)
DATA_DIRECTORY = 'data'
MANUAL = {
    'apache-camel.zip': 'https://drive.google.com/file/d/1qyCflQ9DFoSpUcc6TotW-8L8uNV7djr2/view?usp=sharing',
    'apache-cxf.zip': 'https://drive.google.com/file/d/1sJ1F_RuAVSa-iQbWQLHEQkxtNYpoZKJF/view?usp=sharing',
    'hibernate.zip': 'https://drive.google.com/file/d/1yxyUod2XjjK5fADJDNFZo9NjAIJiTDaI/view?usp=sharing'
}
ALL_ARCHIVES = ARCHIVES + tuple(MANUAL.keys())


def download():
    os.makedirs(DATA_DIRECTORY, exist_ok=True)
    for archive in ARCHIVES:
        full_path = os.path.join(DATA_DIRECTORY, archive)
        if not os.path.exists(full_path):
            url = f'{REPO_LINK}/{archive}'
            print(f'Downloading {archive} from {url}...')
            urllib.request.urlretrieve(url, full_path)
    print('Done!')
    manual = []
    for archive, url in MANUAL.items():
        if not os.path.exists(os.path.join(DATA_DIRECTORY, archive)):
            manual.append((archive, url))
    if manual:
        print('Please download the following archives manually:')
        for i, (archive, url) in enumerate(manual, start=1):
            print(f'{i}. {archive} -- {url}')
    return not bool(manual)


def unpack():
    for archive in ALL_ARCHIVES:
        source = os.path.join(DATA_DIRECTORY, archive)
        print(f'Unpacking {source}...')
        with zipfile.ZipFile(source, 'r') as zf:
            zf.extractall(DATA_DIRECTORY)
    print('Done!')


def cleanup():
    for archive in ALL_ARCHIVES:
        path = os.path.join(DATA_DIRECTORY, archive.removesuffix('.zip'))
        for filename in os.listdir(path):
            if filename.endswith('.txt'):
                full_path = os.path.join(path, filename)
                print('Examining', full_path)
                with open(full_path, 'r') as f:
                    data = [x.strip() for x in f]
                old = data.copy()
                new = [x.removesuffix(';') for x in data]
                if new != old:
                    with open(full_path, 'w') as f:
                        print('Changing', full_path)
                        f.write('\n'.join(new))
    print('Done!')

def main():
    if not download():
        return
    unpack()
    cleanup()


if __name__ == '__main__':
    main()
