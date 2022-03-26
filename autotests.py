#!/usr/bin/python3

import subprocess
import sys
import re
from time import sleep

base_bin = sys.argv[1]
timeout = "22"
print("Running nodes...")
main_node = subprocess.Popen(["python3", "test_node_runner.py", timeout, base_bin, "--cert=tls/cert.pem", "--key=tls/key.pem", "--period=5", "--port=8080", "--bind=127.0.0.1"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
sleep(0.01)
second_node = subprocess.Popen(["python3", "test_node_runner.py", timeout, base_bin, "--cert=tls/cert.pem", "--key=tls/key.pem", "--period=7", "--port=8081", "--bind=127.0.0.1", "--connect=127.0.0.1:8080"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
sleep(0.01)
third_node = subprocess.Popen(["python3", "test_node_runner.py", timeout, base_bin, "--cert=tls/cert.pem", "--key=tls/key.pem", "--period=9", "--port=8082", "--bind=127.0.0.1", "--connect=127.0.0.1:8081"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)

main_out, second_out, third_out = main_node.communicate()[0].decode("utf8"), second_node.communicate()[0].decode("utf8"), third_node.communicate()[0].decode("utf8")

print(main_out)
print(second_out)
print(third_out)

print("Checking their binds...", end="", flush=True)
assert main_out.count("listening on https://127.0.0.1:8080") == 1
assert second_out.count("listening on https://127.0.0.1:8081") == 1
assert third_out.count("listening on https://127.0.0.1:8082") == 1
print("OK")

print("Checking their connects...", end="", flush=True)
assert main_out.count("Connected to") == 0
assert second_out.count("Connected to `127.0.0.1:8080`") == 1 and second_out.count("Connected to") == 1
assert third_out.count("Connected to `127.0.0.1:8081`") == 1 and third_out.count("Connected to") == 1
print("OK")


print("Checking received messages...", end="", flush=True)
assert main_out.count("from `127.0.0.1:8081`") == 3 and main_out.count("from `127.0.0.1:8082`") == 2
assert second_out.count("from `127.0.0.1:8080`") == 4 and second_out.count("from `127.0.0.1:8082`") == 2
assert third_out.count("from `127.0.0.1:8080`") == 4 and third_out.count("from `127.0.0.1:8081`") == 3
print("OK")

print("Checking the number of sent messages...", end="", flush=True)
assert main_out.count('to ["127.0.0.1:8081", "127.0.0.1:8082"]') == 4
assert second_out.count('to ["127.0.0.1:8080", "127.0.0.1:8082"]') == 3
assert third_out.count('to ["127.0.0.1:8080", "127.0.0.1:8081"]') == 2
print("OK")

print("Checking the content of sent messages...", end="", flush=True)
main_msgs = re.findall(r"Sending message `([a-zA-Z0-9]+?)` to \[\"127\.0\.0\.1:8081\", \"127\.0\.0\.1:8082\"]", main_out)
second_msgs = re.findall(r"Sending message `([a-zA-Z0-9]+?)` to \[\"127\.0\.0\.1:8080\", \"127\.0\.0\.1:8082\"]", second_out)
third_msgs = re.findall(r"Sending message `([a-zA-Z0-9]+?)` to \[\"127\.0\.0\.1:8080\", \"127\.0\.0\.1:8081\"]", third_out)

assert main_msgs == re.findall(r"Received message `([a-zA-Z0-9]+?)` from `127\.0\.0\.1:8080`", second_out)
assert main_msgs == re.findall(r"Received message `([a-zA-Z0-9]+?)` from `127\.0\.0\.1:8080`", third_out)

assert second_msgs == re.findall(r"Received message `([a-zA-Z0-9]+?)` from `127\.0\.0\.1:8081`", main_out)
assert second_msgs == re.findall(r"Received message `([a-zA-Z0-9]+?)` from `127\.0\.0\.1:8081`", third_out)

assert third_msgs == re.findall(r"Received message `([a-zA-Z0-9]+?)` from `127\.0\.0\.1:8082`", main_out)
assert third_msgs == re.findall(r"Received message `([a-zA-Z0-9]+?)` from `127\.0\.0\.1:8082`", second_out)
print("OK")
