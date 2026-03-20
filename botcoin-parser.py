import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

doc = data['doc']
paragraphs = doc.split('\n\n')

def get_p(n):
    for p in paragraphs:
        if p.startswith(f'[paragraph_{n}]'):
            return p
    return ""

def find_num(text):
    m = re.search(r'\b(\d+(?:\.\d+)?)\b', text.replace(',', ''))
    if m: return float(m.group(1))
    return None

print("Parsed.")
