import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

doc = data['doc']
companies = data['companies']

print("Ready to process.")
