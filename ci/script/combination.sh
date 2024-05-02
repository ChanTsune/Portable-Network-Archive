#!/usr/bin/env bash
set -eu

keepOptions=( "--keep-dir" "--keep-timestamp" "--keep-permission" "--keep-xattr" )
compressionOptions=( "--store" "--deflate" "--zstd" "--xz" )
encryptionOptions=( "" "--aes cbc" "--aes ctr" "--camellia cbc" "--camellia ctr" )
solidOption=( "" "--solid" )

for keep in "${keepOptions[@]}"; do
  for compress in "${compressionOptions[@]}"; do
    for encrypt in "${encryptionOptions[@]}"; do
      for solid in "${solidOption[@]}"; do
        if [[ "$encrypt" == "--aes cbc" || "$encrypt" == "--aes ctr" || "$encrypt" == "--camellia cbc" || "$encrypt" == "--camellia ctr" ]]; then
          echo "pna experimental stdio create $keep $compress $encrypt --password password $solid -r . --unstable --exclude ./target/ | pna experimental stdio extract --password password --out-dir '/tmp/$keep$compress$encrypt$solid.pna'"
          pna experimental stdio create $keep $compress $encrypt --password password $solid -r . --unstable --exclude ./target/ | pna experimental stdio extract --password password --overwrite --out-dir "/tmp/$keep$compress$encrypt$solid.pna"
        else
          echo "pna experimental stdio create $keep $compress $encrypt $solid -r . --unstable --exclude ./target/ | pna experimental stdio extract --out-dir '/tmp/$keep$compress$encrypt$solid.pna'"
          pna experimental stdio create $keep $compress $encrypt $solid -r . --unstable --exclude ./target/ | pna experimental stdio extract --overwrite --out-dir "/tmp/$keep$compress$encrypt$solid.pna"
        fi
      done
    done
  done
done
