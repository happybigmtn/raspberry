import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

paragraphs = data['doc'].split('\n\n')

# First, extract absolute values
revs = {c: [None, None, None, None] for c in data['companies']}

# Let's extract explicit revenues
def parse_revs():
    # Byte Grid
    revs['Byte Grid'] = [1341, 4748, 1759, 129]
    # Sola Analytics
    revs['Sola Analytics'] = [31, 4253, 3086, 3718]
    # Vela Synth
    revs['Vela Synth'] = [2991, 2082, 3, 1005]
    # Quil Ventures
    revs['Quil Ventures'] = [824, 1952, 4144, 4898]

    # Quil Works
    # Q1: 1221 less than Vela Synth Q1 = 2991 - 1221 = 1770
    revs['Quil Works'][0] = 1770
    # Q2: mid-year revenue exceeded Vela Synth's by 1190 = 2082 + 1190 = 3272
    revs['Quil Works'][1] = 3272
    # Q3: sat 583M under Quil Ventures Q3 = 4144 - 583 = 3561
    revs['Quil Works'][2] = 3561
    # Q4: earned 4675 less than Quil Ventures Q4 = 4898 - 4675 = 223
    revs['Quil Works'][3] = 223

    # Giga Ventures
    # Q1: Quil Ventures outearned by 3M = 824 - 3 = 821
    revs['Giga Ventures'][0] = 821
    # Q2: earning 2140M more than Quil Ventures Q2 = 1952 + 2140 = 4092
    revs['Giga Ventures'][1] = 4092
    # Q3: gap between Giga and Quil Works was 219, with Giga leading = 3561 + 219 = 3780
    revs['Giga Ventures'][2] = 3780
    # Q4: Giga led Quil Works by 3192 = 223 + 3192 = 3415
    revs['Giga Ventures'][3] = 3415

    # Tala Forge
    # Q1: beat Quil Ventures by 3390 = 824 + 3390 = 4214
    revs['Tala Forge'][0] = 4214
    # Q2: surpassed Quil Ventures by 2325 = 1952 + 2325 = 4277
    revs['Tala Forge'][1] = 4277
    # Q3: difference between Tala Forge and Byte Grid came to 610 in Tala Forge's favor = 1759 + 610 = 2369
    revs['Tala Forge'][2] = 2369
    # Q4: surpassed Byte Grid by 861 = 129 + 861 = 990
    revs['Tala Forge'][3] = 990

    # Byte Solutions
    # Q1: reported 1906 less than Tala Forge = 4214 - 1906 = 2308
    revs['Byte Solutions'][0] = 2308
    # Q2: shortfall relative to Tala Forge was 4027 = 4277 - 4027 = 250
    revs['Byte Solutions'][1] = 250
    # Q3: trailed Sola Analytics by 1943 = 3086 - 1943 = 1143
    revs['Byte Solutions'][2] = 1143
    # Q4: Sola Analytics outearned Byte Solutions by 3133 = 3718 - 3133 = 585
    revs['Byte Solutions'][3] = 585

    # Tera Sys
    # Q1: reported revenue 3191 ahead of Byte Grid = 1341 + 3191 = 4532
    revs['Tera Sys'][0] = 4532
    # Q2: reported 4592 less than Byte Grid = 4748 - 4592 = 156
    revs['Tera Sys'][1] = 156
    # Q3: shortfall relative to Quil Ventures was 2665 = 4144 - 2665 = 1479
    revs['Tera Sys'][2] = 1479
    # Q4: sat 1249 under Quil Ventures Q4 = 4898 - 1249 = 3649
    revs['Tera Sys'][3] = 3649

    # Opti Capital
    # Q1: lagged Tera Sys by 3409 = 4532 - 3409 = 1123
    revs['Opti Capital'][0] = 1123
    # Q2: surpassed Tera Sys by 2518 = 156 + 2518 = 2674
    revs['Opti Capital'][1] = 2674
    # Q3: led Quil Ventures by 284 = 4144 + 284 = 4428
    revs['Opti Capital'][2] = 4428
    # Q4: lagged Quil Ventures by 3062 = 4898 - 3062 = 1836
    revs['Opti Capital'][3] = 1836

    # Vela Bio
    # Q1: sat 779 under Byte Grid = 1341 - 779 = 562
    revs['Vela Bio'][0] = 562
    # Q2: fell 3749 short of Byte Grid = 4748 - 3749 = 999
    revs['Vela Bio'][1] = 999
    # Q3: ran 649 above Vela Synth = 3 + 649 = 652
    revs['Vela Bio'][2] = 652
    # Q4: came in 3024 above Vela Synth = 1005 + 3024 = 4029
    revs['Vela Bio'][3] = 4029

    # Nova Solutions
    # Q1: surpassed Vela Bio by 2779 = 562 + 2779 = 3341
    revs['Nova Solutions'][0] = 3341
    # Q2: earned 954 less than Vela Bio = 999 - 954 = 45
    revs['Nova Solutions'][1] = 45
    # Q3: posted growth 30 points stronger than Quil Works... wait, late-year results came in 748 above Quil Works = 3561 + 748 = 4309
    revs['Nova Solutions'][2] = 4309
    # Q4: exceeded Quil Works by 1055 = 223 + 1055 = 1278
    revs['Nova Solutions'][3] = 1278

    # Kova Wave
    # Q1: 1386 below Vela Synth = 2991 - 1386 = 1605
    revs['Kova Wave'][0] = 1605
    # Q2: exceeded Vela Synth by 422 = 2082 + 422 = 2504
    revs['Kova Wave'][1] = 2504
    # Q3: late-year revenue beat Byte Grid by 3237 = 1759 + 3237 = 4996
    revs['Kova Wave'][2] = 4996
    # Q4: gap between Kova and Byte Grid was 3784 with Kova leading = 129 + 3784 = 3913
    revs['Kova Wave'][3] = 3913

    # Fuse Sciences
    # Q1: reported 638 less than Kova Wave = 1605 - 638 = 967
    revs['Fuse Sciences'][0] = 967
    # Q2: exceeded Kova Wave by 1550 = 2504 + 1550 = 4054
    revs['Fuse Sciences'][1] = 4054
    # Q3: came in 610 behind Giga Ventures = 3780 - 610 = 3170
    revs['Fuse Sciences'][2] = 3170
    # Q4: trailing Giga Ventures by 1643 = 3415 - 1643 = 1772
    revs['Fuse Sciences'][3] = 1772

    # Zeta Hub
    # Q1: ran 442 above Fuse Sciences = 967 + 442 = 1409
    revs['Zeta Hub'][0] = 1409
    # Q2: 1026 below Fuse Sciences = 4054 - 1026 = 3028
    revs['Zeta Hub'][1] = 3028
    # Q3: late-year revenue beat Byte Solutions by 3562 = 1143 + 3562 = 4705
    revs['Zeta Hub'][2] = 4705
    # Q4: shortfall relative to Byte Solutions was 132 = 585 - 132 = 453
    revs['Zeta Hub'][3] = 453

    # Mesa Synth
    # Q3: 2799 below Zeta Hub = 4705 - 2799 = 1906
    revs['Mesa Synth'][2] = 1906
    # Q4: 1612 north of Zeta Hub = 453 + 1612 = 2065
    revs['Mesa Synth'][3] = 2065

    # Zyra Sys
    # Q1: led Kova Wave by 2113 = 1605 + 2113 = 3718
    revs['Zyra Sys'][0] = 3718
    # Q2: earned 1974 less than Kova Wave = 2504 - 1974 = 530
    revs['Zyra Sys'][1] = 530
    # Q3: pulled in 1259 more than Vela Synth = 3 + 1259 = 1262
    revs['Zyra Sys'][2] = 1262
    # Q4: gap between Zyra and Vela Synth was 3665 with Zyra leading = 1005 + 3665 = 4670
    revs['Zyra Sys'][3] = 4670

    # Fuse Tech
    # Q1: surpassed Byte Grid by 3496 = 1341 + 3496 = 4837
    revs['Fuse Tech'][0] = 4837
    # Q2: earned 1638 less than Byte Grid = 4748 - 1638 = 3110
    revs['Fuse Tech'][1] = 3110
    # Q3: trailing Byte Solutions by 724 = 1143 - 724 = 419
    revs['Fuse Tech'][2] = 419
    # Q4: exceeded Byte Solutions by 2463 = 585 + 2463 = 3048
    revs['Fuse Tech'][3] = 3048

    # Fuse Edge
    # Q1: surpassed Vela Synth by 1588 = 2991 + 1588 = 4579
    revs['Fuse Edge'][0] = 4579
    # Q2: earned 985 less than Vela Synth = 2082 - 985 = 1097
    revs['Fuse Edge'][1] = 1097
    # Q3: pulled in 1731 more than Fuse Tech = 419 + 1731 = 2150
    revs['Fuse Edge'][2] = 2150
    # Q4: fell 2666 short of Fuse Tech = 3048 - 2666 = 382
    revs['Fuse Edge'][3] = 382

    # Mesa Synth (rest)
    # Q1: 819 less than Fuse Edge = 4579 - 819 = 3760
    revs['Mesa Synth'][0] = 3760
    # Q2: led Fuse Edge by 3306 = 1097 + 3306 = 4403
    revs['Mesa Synth'][1] = 4403

    # Pyra Materials
    # Q1: came in 2624 behind Fuse Edge = 4579 - 2624 = 1955
    revs['Pyra Materials'][0] = 1955
    # Q2: recorded 149 less than Fuse Edge = 1097 - 149 = 948
    revs['Pyra Materials'][1] = 948
    # Q3: Tera Sys outearned Pyra by 1350 = 1479 - 1350 = 129
    revs['Pyra Materials'][2] = 129
    # Q4: Pyra 198 behind Tera Sys = 3649 - 198 = 3451
    revs['Pyra Materials'][3] = 3451

parse_revs()

for c in revs:
    print(f"{c}: {revs[c]}")
    if None not in revs[c]:
        diff = max(revs[c]) - min(revs[c])
        print(f"  Volatility: {diff}")
