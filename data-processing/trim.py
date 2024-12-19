import json


filenames = [
    ('apache-camel-old.json', 'apache-camel.json'),
    ('apache-cxf-old.json', 'apache-cxf.json'),
]

for filename_old, filename_new in filenames:
    with open(filename_old, 'r') as f:
        data = json.load(f)
    to_delete_major = []
    for major, minors in data.items():
        to_delete = []
        for minor, values in minors.items():
            if not values['commit_data']:
                to_delete.append(minor)
        for minor in to_delete:
            print(f'Deleting {minor} from {major}')
            del minors[minor]
        if not minors:
            to_delete_major.append(major)
    for major in to_delete_major:
        print(f'Deleting {major}')
        del data[major]
    with open(filename_new, 'w') as f:
        json.dump(data, f, indent=2)
