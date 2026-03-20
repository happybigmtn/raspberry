import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

paragraphs = data['doc'].split('\n\n')

for p in paragraphs:
    if 'debt-to-equity' in p.lower() or 'd/e' in p.lower() or 'leverage' in p.lower():
        print(p)
