cd ..
cd data-processing
shopt -s globstar
python dummy.py -i ../data/triples-gnn/**/*.json -o ../data/results/dummy --gnn
#python dummy.py -i ../data/triples-gnn/**/*.json -o ../data/results/dummy/dummy_undirected.json --undirected