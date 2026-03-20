import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

paragraphs = data['doc'].split('\n\n')

# Find all growth references
for p in paragraphs:
    if 'growth' in p.lower() and ('q3' in p.lower() or 'third' in p.lower() or 'late' in p.lower()):
        print(p)

