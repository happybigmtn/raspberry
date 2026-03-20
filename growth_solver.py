import json

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

growth = {c: None for c in data['companies']}
growth['Byte Grid'] = -3
growth['Sola Analytics'] = -26
growth['Vela Synth'] = -23
growth['Quil Ventures'] = -28

growth['Tala Forge'] = -3 + 7
growth['Byte Solutions'] = -26 + 3
growth['Fuse Tech'] = growth['Byte Solutions'] + 50
growth['Fuse Edge'] = growth['Fuse Tech'] - 36
growth['Tera Sys'] = -28 + 68
growth['Pyra Materials'] = growth['Tera Sys'] - 64
growth['Quil Works'] = -28 + 47
growth['Giga Ventures'] = growth['Quil Works'] - 18
growth['Fuse Sciences'] = growth['Giga Ventures'] + 48
growth['Nova Solutions'] = growth['Quil Works'] + 30
growth['Kova Wave'] = -3 + 38
growth['Zeta Hub'] = growth['Byte Solutions'] + 14
growth['Mesa Synth'] = growth['Zeta Hub'] + 59
growth['Zyra Sys'] = -23 + 40
growth['Opti Capital'] = -28 + 18
growth['Vela Bio'] = -23 + 33

for k, v in growth.items():
    print(f"{k}: Q3 Growth = {v}")
