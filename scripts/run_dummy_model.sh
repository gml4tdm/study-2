cd ..
cd py_scripts
shopt -s globstar
python dummy.py -i ../data/triples/**/*.json -o ../data/results/dummy/dummy.json
python dummy.py -i ../data/triples/**/*.json -o ../data/results/dummy/dummy_undirected.json --undirected