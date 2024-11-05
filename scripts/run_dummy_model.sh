cd ..
cd models
#python dummy.py -i ../data/triples/apache-ant/*.json -o ../data/results/dummy/apache-ant.json
#python dummy.py -i ../data/triples/apache-camel/*.json -o ../data/results/dummy/apache-camel.json
#python dummy.py -i ../data/triples/apache-cxf/*.json -o ../data/results/dummy/apache-cxf.json
#python dummy.py -i ../data/triples/apache-derby/*.json -o ../data/results/dummy/apache-derby.json
#python dummy.py -i ../data/triples/hibernate/*.json -o ../data/results/dummy/hibernate.json
shopt -s globstar
python dummy.py -i ../data/triples/**/*.json -o ../data/results/dummy/dummy.json
python dummy.py -i ../data/triples/**/*.json -o ../data/results/dummy/dummy_undirected.json --undirected