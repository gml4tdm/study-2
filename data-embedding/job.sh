#!/bin/bash
#SBATCH --job-name=codet5-data-embedding
#SBATCH --nodes=1
#SBATCH --ntasks-per-node=1
#SBATCH --cpus-per-task=1
#SBATCH --mem=32G
#SBATCH --time=6:00:00
#SBATCH --output=logs/codet5-data-embedding.out
#SBATCH --gpus-per-node=1
#SBATCH --signal=B:12@300

module load PyTorch/2.1.2-foss-2023a-CUDA-12.1.1

mkdir -p $TMPDIR/source-code
mkdir -p $TMPDIR/embeddings
cp /scratch/$USER/source-code.tar.gz
tar xzf source-code.tar.gz -C $TMPDIR/source-code

trap 'tar czvf embeddings.tar.gz $TMPDIR/embeddings; cp embeddings.tar.gz /scratch/$USER/embeddings.tar.gz' 12

python base.py \
    --input-directory $TMPDIR/source-code \
    --output-directory $TMPDIR/embeddings \
    --interactive \
    --batch-limit 512

tar czvf embeddings.tar.gz $TMPDIR/embeddings
cp embeddings.tar.gz /scratch/$USER/embeddings.tar.gz
