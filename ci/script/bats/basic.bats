#!/usr/bin/env bats

@test "basic" {
  run true
  [ "$status" -eq 0 ]
}