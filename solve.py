import json
import re
import math

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

doc = data['doc']
paragraphs = doc.split('\n\n')

def get_p(n):
    for p in paragraphs:
        if p.startswith(f'[paragraph_{n}]'):
            return p
    return ""

print("Parsed challenge.")
