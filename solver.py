import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

paragraphs = data['doc'].split('\n\n')
companies = data['companies']

metrics = {c: {'rev':{}, 'growth':{}, 'emp':None, 'de':None, 'sat':None, 'pub':None, 'founded':None, 'ceo':None, 'city':None, 'country':None} for c in companies}

def extract_number(text):
    m = re.search(r'([\d,]+(?:\.\d+)?)', text)
    if m:
        return float(m.group(1).replace(',', ''))
    return None

for p in paragraphs:
    for c in companies:
        if c.lower() in p.lower() or c.replace(' ', '').lower() in p.replace(' ', '').lower():
            # Company meta
            if 'traces its origins to' in p or 'was founded in' in p:
                m1 = re.search(r'(?:traces its origins to|was founded in) (\d{4})', p)
                if m1: metrics[c]['founded'] = int(m1.group(1))
                if 'went public' in p: metrics[c]['pub'] = True
                if 'remains privately held' in p: metrics[c]['pub'] = False
                
                m2 = re.search(r'based in ([A-Z][a-z]+), ([A-Z][a-z]+)', p)
                if m2:
                    metrics[c]['city'] = m2.group(1)
                    metrics[c]['country'] = m2.group(2)
                
                m3 = re.search(r'run by ([A-Z][a-z]+) ([A-Z][a-z]+)', p)
                if m3: metrics[c]['ceo'] = m3.group(2)
                
                m4 = re.search(r'led by ([A-Z][a-z]+) ([A-Z][a-z]+)', p)
                if m4: metrics[c]['ceo'] = m4.group(2)

            # Explicit metrics
            if 'employs' in p and 'debt-to-equity' in p and 'satisfaction' in p:
                if c in p:
                    m1 = re.search(r'employs (?:about )?([\d,]+) staff', p)
                    if m1: metrics[c]['emp'] = float(m1.group(1).replace(',', ''))
                    m2 = re.search(r'debt-to-equity ratio of ([\d\.]+)', p)
                    if m2: metrics[c]['de'] = float(m2.group(1))
                    m3 = re.search(r'satisfaction score of ([\d\.]+)', p)
                    if m3: metrics[c]['sat'] = float(m3.group(1))

            # Explicit revenue
            if 'Across the fiscal year' in p and c in p:
                # Need to parse Q1, Q2, Q3, Q4
                pass

print(json.dumps(metrics, indent=2))
