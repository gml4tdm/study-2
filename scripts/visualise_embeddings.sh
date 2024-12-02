cd ../data-processing

declare -a arr=(
  "apache-ant/1.1/src/main"
  "apache-ant/1.2/src/main"
  "apache-ant/1.3/src/main"
  "apache-ant/1.4/src/main"
  "apache-ant/1.5/src/main"
  "apache-ant/1.5.2/src/main"
  "apache-ant/1.6.0/src/main"
  "apache-ant/1.7.0/src/main"
  "apache-ant/1.8.0/src/main"
  "apache-ant/1.9.0/src/main"
  "apache-ant/1.10.0/src/main"
  "apache-camel/2.0.0/camel-core/src/main/java"
  "apache-camel/2.1.0/camel-core/src/main/java"
  "apache-camel/2.2.0/camel-core/src/main/java"
  "apache-camel/2.3.0/camel-core/src/main/java"
  "apache-camel/2.4.0/camel-core/src/main/java"
  "apache-camel/2.5.0/camel-core/src/main/java"
  "apache-camel/2.6.0/camel-core/src/main/java"
  "apache-camel/2.7.0/camel-core/src/main/java"
  "apache-camel/2.8.0/camel-core/src/main/java"
  "apache-camel/2.9.0/camel-core/src/main/java"
  "apache-camel/2.10.0/camel-core/src/main/java"
  "apache-camel/2.11.0/camel-core/src/main/java"
  "apache-camel/2.12.0/camel-core/src/main/java"
  "apache-camel/2.13.0/camel-core/src/main/java"
  "apache-camel/2.14.0/camel-core/src/main/java"
  "apache-camel/2.15.0/camel-core/src/main/java"
  "apache-camel/2.16.0/camel-core/src/main/java"
  "apache-camel/2.17.0/camel-core/src/main/java"
)

for i in "${arr[@]}"
do
  src_root="../data/compacted-projects/$i"
  emb_root="../data/embeddings/output/compacted-projects/$i"
  vis_dir="$(echo $i | cut -f 1,2 -d / | tr / -)"
  vis_root="../data/figures/embeddings/$vis_dir"

  echo "Generating embedding visualisation for $vis_dir"

  python visualise_embeddings.py -s $src_root -e $emb_root -m euclidean -l class -o $vis_root/class_euclidean.png
  python visualise_embeddings.py -s $src_root -e $emb_root -m cosine -l class -o $vis_root/class_cosine.png
  python visualise_embeddings.py -s $src_root -e $emb_root -m euclidean -l package -o $vis_root/package_euclidean.png
  python visualise_embeddings.py -s $src_root -e $emb_root -m cosine -l package -o $vis_root/package_cosine.png
done
