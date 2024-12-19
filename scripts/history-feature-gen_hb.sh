#!/bin/bash
#SBATCH --job-name=svm-hist
#SBATCH --nodes=1
#SBATCH --ntasks-per-node=1
#SBATCH --cpus-per-task=1
#SBATCH --mem=32G
#SBATCH --time=90:00
#SBATCH --output=log.out

module load PyTorch/2.1.2-foss-2023a-CUDA-12.1.1
source $HOME/venvs/torch-1-2-1/bin/activate

cd pipeline
shopt -s extglob
shopt -s globstar

echo "Making directories"
mkdir $TMPDIR/temp
#mkdir /scratch/p311400/triples
#mkdir /scratch/p311400/graph-changes
#mkdir /scratch/p311400/co-changes

#echo "Generating Triples"
#./target/release/pipeline generate-train-test-triples \
#  -i /scratch/p311400/graphs/apache-ant/!(*1.5.2).odem \
#  -o /scratch/p311400/triples/apache-ant \
#  --language java \
#  --only-common-nodes-for-training
#./target/release/pipeline generate-train-test-triples \
#  -i /scratch/p311400/graphs/apache-camel/*.odem \
#  -o /scratch/p311400/triples/apache-camel \
#  --language java \
#  --only-common-nodes-for-training
#./target/release/pipeline generate-train-test-triples \
#  -i /scratch/p311400/graphs/apache-cxf/*.odem \
#  -o /scratch/p311400/triples/apache-cxf \
#  --language java \
#  --only-common-nodes-for-training
#
#echo "Time Series Features"
#./target/release/pipeline generate-time-series-features \
#    --input-files /scratch/p311400/graphs/apache-ant/!(*1.5.2).odem \
#    --output-file /scratch/p311400/graph-changes/apache-ant.json
#./target/release/pipeline generate-time-series-features \
#    --input-files /scratch/p311400/graphs/apache-camel/*.odem \
#    --output-file /scratch/p311400/graph-changes/apache-camel.json
#./target/release/pipeline generate-time-series-features \
#    --input-files /scratch/p311400/graphs/apache-cxf/*.odem \
#    --output-file /scratch/p311400/graph-changes/apache-cxf.json

echo "Processing history"
./target/release/pipeline process-history \
    --input-file /scratch/p311400/history/apache-ant.json \
    --output-file $TMPDIR/temp/apache-ant.json
./target/release/pipeline process-history \
    --input-file /scratch/p311400/history/apache-camel.json \
    --output-file $TMPDIR/temp/apache-camel.json
./target/release/pipeline process-history \
    --input-file /scratch/p311400/history/apache-cxf.json \
    --output-file $TMPDIR/temp/apache-cxf.json

echo "Processing Co-change data"
./target/release/pipeline generate-co-change-features \
    --input-file $TMPDIR/temp/apache-ant.json \
    --output-file $TMPDIR/temp/apache-ant-co-change.json
./target/release/pipeline generate-co-change-features \
    --input-file $TMPDIR/temp/apache-camel.json \
    --output-file $TMPDIR/temp/apache-camel-co-change.json
./target/release/pipeline generate-co-change-features \
    --input-file $TMPDIR/temp/apache-cxf.json \
    --output-file $TMPDIR/temp/apache-cxf-co-change.json

echo "Finalising Co-change data"
./target/release/pipeline finalise-co-change-features \
    --change-file $TMPDIR/temp/apache-ant-co-change.json \
    --graph-files /scratch/p311400/graphs/apache-ant/!(*1.5.2).odem \
    --output-file /scratch/p311400/co-changes/apache-ant.json
./target/release/pipeline finalise-co-change-features \
    --change-file $TMPDIR/temp/apache-camel-co-change.json \
    --graph-files /scratch/p311400/graphs/apache-camel/*.odem \
    --output-file /scratch/p311400/co-changes/apache-camel.json
./target/release/pipeline finalise-co-change-features \
    --change-file $TMPDIR/temp/apache-cxf-co-change.json \
    --graph-files /scratch/p311400/graphs/apache-cxf/*.odem \
    --output-file /scratch/p311400/co-changes/apache-cxf.json

echo "Running Model"
cd ../data-processing
python svm_time.py \
  -t /scratch/p311400/triples/**/*.json \
  -g /scratch/p311400/graph-changes/*.json \
  -c /scratch/p311400/co-changes/*.json \
  -m /scratch/p311400/graphs/**/*.json \
  -o $HOME/result.json \
  -l apache-ant apache-camel apache-cxf

echo "Done"