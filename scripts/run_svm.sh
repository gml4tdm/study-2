cd ..
cd py_scripts
shopt -s globstar
python svm.py -i ../data/triples/**/*.json -f ../data/graphs -o ../data/results/svm/replication.json