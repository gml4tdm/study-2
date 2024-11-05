cd ..
cd models
shopt -s globstar
python svm.py -i ../data/triples/**/*.json -f ../data/graphs -o ../data/results/svm/relication.json