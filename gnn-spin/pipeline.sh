if ! command -v cargo version 2>&1> /dev/null then
  echo "<cargo> could not be found!"
  exit 1
fi

if [ ! -d "../../graphs" ]; then
  echo "Directory ../../graphs does not exist."
  echo "Please run replication/prepare_data.py first."
  exit 1
fi

echo "Downloading source code"
cd downloader
cargo run --release -- ../../versions.json ../../source-code
cd ..

echo "Compacting source code"
cd compacter
cargo run --release -- \
  ../../source-code \
  ../../graphs \
  ../../compacted-projects
cd ..

echo "Deleting source code to save disk space"
rm -rf ../../soruce code

echo "Extracting file structure information"
cd structure
cargo run --release -- ../../compacted-projects ../../structure
cd ..
