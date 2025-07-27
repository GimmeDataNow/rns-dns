#! /usr/bin/env bash

source ~/.bashrc

SESH="rns-dns-dev"

tmux has-session -t $SESH 2>/dev/null


if [ $? != 0 ]; then

  tmux new-session -d -s $SESH -n "edit-server"
  tmux send-keys -t $SESH:edit-server "nix develop" C-m C-l
  tmux send-keys -t $SESH:edit-server "cd ./src" C-m C-l

  tmux new-window -t $SESH -n "edit-client"
  tmux send-keys -t $SESH:edit-client "nix develop" C-m C-l
  tmux send-keys -t $SESH:edit-client "cd ./src" C-m C-l

  tmux new-window -t $SESH -n "rnsd"
  tmux send-keys -t $SESH:rnsd "nix develop" C-m C-l
  tmux send-keys -t $SESH:rnsd "rnsd -vv"

  tmux new-window -t $SESH -n "debug-cli"
  tmux send-keys -t $SESH:debug-cli "nix develop" C-m C-l

  tmux new-window -t $SESH -n "e1"
  tmux send-keys -t $SESH:e1 "nix develop" C-m C-l
  tmux send-keys -t $SESH:e1 "cargo run --example hello-client"

  tmux new-window -t $SESH -n "e2"
  tmux send-keys -t $SESH:e2 "nix develop" C-m C-l
  tmux send-keys -t $SESH:e2 "cargo run --example hello-server"

  tmux new-window -t $SESH -n "e3"
  tmux send-keys -t $SESH:e3 "nix develop" C-m C-l


fi


tmux attach-session -t $SESH
