#!/usr/bin/python3
import subprocess
import sys
from time import sleep
import os

os.environ['RUST_LOG'] = "simplep2pgossip=info,warp=info"
timeout = float(sys.argv[1])
node = subprocess.Popen([sys.argv[2], *sys.argv[3:]], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
sleep(timeout)
node.terminate()
print(node.communicate()[0].decode("utf8"))