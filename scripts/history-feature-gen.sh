cd ../pipeline
shopt -s extglob

echo "Time Series Features"
./target/release/pipeline generate-time-series-features \
    --input-files ../data/graphs/apache-ant/!(*1.5.2).odem \
    --output-file ../data/svm-time-graph-changes/apache-ant.json

#echo "Processing history"
#./target/release/pipeline process-history \
#    --input-file ../data/history/ant.json \
#    --output-file ../data/time-features/apache-ant/temp/history-processed.json

#echo "Processing Co-change data"
#./target/release/pipeline generate-co-change-features \
#    --input-file ../data/time-features/apache-ant/temp/history-processed.json \
#    --output-file ../data/time-features/apache-ant/temp/co-change.json

#echo "Finalising Co-change data"
#./target/release/pipeline finalise-co-change-features \
#    --change-file ../data/time-features/apache-ant/temp/co-change.json \
#    --graph-files ../data/graphs/apache-ant/!(*1.5.2).odem \
#    --output-file ../data/time-features/apache-ant/co-change.json