#!/usr/bin/env bash

##
# Usage:
#   ./so_inject.sh inject <PID> </full/path/to/lib.so>
#   ./so_inject.sh uninject <PID> <HANDLE_ADDRESS>
#
# Example:
#   ./so_inject.sh inject 1234 /home/user/mylib.so
#      --> prints the handle address, e.g. 0x7ffff7fd1000
#   ./so_inject.sh uninject 1234 0x7ffff7fd1000
##

ACTION=$1
PID=$2

case "$ACTION" in
  inject)
    LIBPATH=$3
    if [ -z "$PID" ] || [ -z "$LIBPATH" ]; then
      echo "Usage: $0 inject <PID> <LIBPATH>"
      exit 1
    fi
    
    # Attach to PID, run dlopen, grab the returned handle, and print it out.
    echo "[+] Injecting '$LIBPATH' into PID $PID ..."
    gdb -p "$PID" -ex "call (void*)dlopen(\"$LIBPATH\", 1)"
    ;;

  uninject)
    ADDR=$3
    if [ -z "$PID" ] || [ -z "$ADDR" ]; then
      echo "Usage: $0 uninject <PID> <HANDLE_ADDRESS>"
      exit 1
    fi

    # Attach to PID, call dlclose on the given handle.
    echo "[+] Uninjecting handle '$ADDR' from PID $PID ..."
    gdb -p "$PID" \
        -batch \
        -ex "set confirm off" \
        -ex "call (int)dlclose((void*)$ADDR)" \
        -ex "detach" \
        -ex "quit"
    ;;
  
  *)
    echo "Usage:"
    echo "  $0 inject <PID> </path/to/lib.so>"
    echo "  $0 uninject <PID> <HANDLE_ADDRESS>"
    exit 1
    ;;
esac
