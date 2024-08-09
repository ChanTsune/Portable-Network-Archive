#!/usr/bin/env bash
set -eu

keepOptions=( "--keep-dir" "--keep-timestamp" "--keep-permission" "--keep-xattr" )
compressionOptions=( "--store" "--deflate" "--zstd" "--xz" )
encryptionOptions=( "" "--aes cbc" "--aes ctr" "--camellia cbc" "--camellia ctr" )
hashOptions=( "" "--argon2 t=1,m=50" "--pbkdf2 r=1")
solidOption=( "" "--solid" )

for keep in "${keepOptions[@]}"; do
  for compress in "${compressionOptions[@]}"; do
    for encrypt in "${encryptionOptions[@]}"; do
      for solid in "${solidOption[@]}"; do
        if [[ "$encrypt" == "--aes cbc" || "$encrypt" == "--aes ctr" || "$encrypt" == "--camellia cbc" || "$encrypt" == "--camellia ctr" ]]; then
          for hash in "${hashOptions[@]}"; do
            echo "pna experimental stdio -c $keep $compress $encrypt $hash --password password $solid -r . --unstable --exclude ./target/ | pna experimental stdio -x --password password --out-dir '/tmp/$keep$compress$encrypt$hash$solid.pna'"
            pna experimental stdio -c $keep $compress $encrypt $hash --password password $solid -r . --unstable --exclude ./target/ | pna experimental stdio -x --password password --overwrite --out-dir "/tmp/$keep$compress$encrypt$hash$solid.pna"
            diff -r . "/tmp/$keep$compress$encrypt$hash$solid.pna"
          done
        else
          echo "pna experimental stdio -c $keep $compress $encrypt $solid -r . --unstable --exclude ./target/ | pna experimental stdio -x --out-dir '/tmp/$keep$compress$encrypt$solid.pna'"
          pna experimental stdio -c $keep $compress $encrypt $solid -r . --unstable --exclude ./target/ | pna experimental stdio -x --overwrite --out-dir "/tmp/$keep$compress$encrypt$solid.pna"
          diff -r . "/tmp/$keep$compress$encrypt$solid.pna"
        fi
      done
    done
  done
done
