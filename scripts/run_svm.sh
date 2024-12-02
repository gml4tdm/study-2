cd ..
cd data-processing
shopt -s globstar
#python svm.py -i ../data/triples/**/*.json -f ../data/graphs -o ../data/results/svm/replication.json
python svm.py -i ../data/triples/**/*.json -f ../data/graphs -o ../data/results/svm/replication_balanced.json -b