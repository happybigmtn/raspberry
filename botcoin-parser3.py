import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

paragraphs = data['doc'].split('\n\n')

companies = {c: {} for c in data['companies']}

for c in companies:
    # Get all paragraphs for this company
    c_paras = [p for p in paragraphs if c.lower() in p.lower()]
    companies[c]['paras'] = c_paras

for c, info in companies.items():
    print(f"[{c}]")
    for p in info['paras']:
        print(p)
    print("")

