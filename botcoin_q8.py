import json
import re

with open('/tmp/botcoin-challenge.json', 'r') as f:
    data = json.load(f)

paragraphs = data['doc'].split('\n\n')

revs = {c: [None, None, None, None] for c in data['companies']}
emps = {c: None for c in data['companies']}

# Explicit revenues
revs['Tala Prime'] = [3025, 3167, 3957, 627]
revs['Myra Arc'] = [3702, 4549, 1063, 1948]
revs['Flux Data'] = [2165, 3084, 540, 2885]
revs['Myra Global'] = [2152, 4564, 2217, 921]
revs['Opti Materials'] = [2533, 4211, 2519, 606]

# Prim Edge
# Q1 beat Nova Synth by 1937
# Q2 lagged Nova Synth by 1058
# Q3 underperformed Tala Prime by 2912 = 3957 - 2912 = 1045
revs['Prim Edge'][2] = 1045
# Q4 reported 2217 ahead of Tala Prime = 627 + 2217 = 2844
revs['Prim Edge'][3] = 2844

# Nova Synth
# Q1 trailed Tala Prime by 1173 = 3025 - 1173 = 1852
revs['Nova Synth'][0] = 1852
# Q2 recorded 1060 less than Tala Prime = 3167 - 1060 = 2107
revs['Nova Synth'][1] = 2107
# Q3 trailed Opti Materials by 138 = 2519 - 138 = 2381
revs['Nova Synth'][2] = 2381
# Q4 exceeded Opti Materials by 1732 = 606 + 1732 = 2338
revs['Nova Synth'][3] = 2338

# Prim Edge (cont)
# Q1 beat Nova Synth by 1937 = 1852 + 1937 = 3789
revs['Prim Edge'][0] = 3789
# Q2 lagged Nova Synth by 1058 = 2107 - 1058 = 1049
revs['Prim Edge'][1] = 1049

# Rune Flow
# Q1 trailing Nova Synth by 2356 = 1852 - 2356 = ? wait, trailing means Nova Synth is larger. Let's see: Rune Flow trailing Nova Synth by 2356 means Nova = 1852. So Rune Flow Q1 is negative? Ah, maybe revenue is just 1852 - 2356 = -504? Wait, trailing by 2356 means it could be less. Let's re-read: "Revenue data shows Rune Flow trailing Nova Synth by 2356M in the first-quarter."
# Actually, wait. Let's check paragraph 94: Rune Flow trailing Nova Synth by 2356M... wait, Nova Synth Q1 is 1852. Rune Flow trailing by 2356? That's negative.
# Let's check Prim Dynamics vs Myra Arc
# Prim Dynamics came in 446 behind Myra Arc in Q1 = 3702 - 446 = 3256
revs['Prim Dynamics'][0] = 3256
# Q2 lagged Myra Arc by 4026 = 4549 - 4026 = 523
revs['Prim Dynamics'][1] = 523
# Orba Tech earning 2791 less than Prim Dynamics in late-year... wait "Prim Dynamics earning 2791 more than Orba Tech in the late-year"
# Q4 Prim Dynamics outpaced Orba Tech by 4028

# Myra Sys
# Q1 1698 north of Prim Dynamics = 3256 + 1698 = 4954
revs['Myra Sys'][0] = 4954
# Q2 difference came to 3041 in Myra Sys's favor = 523 + 3041 = 3564
revs['Myra Sys'][1] = 3564
# Q3 trailed Nova Synth by 1797 = 2381 - 1797 = 584
revs['Myra Sys'][2] = 584
# Q4 ran 624 above Nova Synth = 2338 + 624 = 2962
revs['Myra Sys'][3] = 2962

# Rune Flow
# Q3 beat Myra Sys's tally by 2360 = 584 + 2360 = 2944
revs['Rune Flow'][2] = 2944
# Q4 earning 285 more than Myra Sys = 2962 + 285 = 3247
revs['Rune Flow'][3] = 3247

# Arlo Dynamics
# Q1 earned 1034 less than Myra Arc = 3702 - 1034 = 2668
revs['Arlo Dynamics'][0] = 2668
# Q2 lower than Myra Arc's by 3411 = 4549 - 3411 = 1138
revs['Arlo Dynamics'][1] = 1138
# Q3 ran 107 above Myra Global = 2217 + 107 = 2324
revs['Arlo Dynamics'][2] = 2324
# Q4 came in 312 behind Myra Global = 921 - 312 = 609
revs['Arlo Dynamics'][3] = 609

# Axio Bio
# Q3 exceeded Myra Arc's by 1739 = 1063 + 1739 = 2802
revs['Axio Bio'][2] = 2802
# Q4 reported 1636 ahead of Myra Arc = 1948 + 1636 = 3584
revs['Axio Bio'][3] = 3584
# Q1 fell 901 short of Opti Materials = 2533 - 901 = 1632
revs['Axio Bio'][0] = 1632
# Q2 sat 480 under Opti Materials = 4211 - 480 = 3731
revs['Axio Bio'][1] = 3731

# Dyna Forge
# Q1 surpassed Flux Data by 1581 = 2165 + 1581 = 3746
revs['Dyna Forge'][0] = 3746
# Q2 shortfall relative to Flux Data was 243 = 3084 - 243 = 2841
revs['Dyna Forge'][1] = 2841
# Q3 north of Opti Materials by 2444 = 2519 + 2444 = 4963
revs['Dyna Forge'][2] = 4963
# Q4 exceeded Opti Materials by 541 = 606 + 541 = 1147
revs['Dyna Forge'][3] = 1147

# Sero Works
# Q1 led Flux Data by 495 = 2165 + 495 = 2660
revs['Sero Works'][0] = 2660
# Q2 underperformed Flux Data by 1640 = 3084 - 1640 = 1444
revs['Sero Works'][1] = 1444
# Q3 north of Opti Materials by 1647 = 2519 + 1647 = 4166
revs['Sero Works'][2] = 4166
# Q4 pulled in 4232 more than Opti Materials = 606 + 4232 = 4838
revs['Sero Works'][3] = 4838

# Orba Tech
# Q1 posted 1485 more than Flux Data = 2165 + 1485 = 3650
revs['Orba Tech'][0] = 3650
# Q2 trailing Flux Data by 1569 = 3084 - 1569 = 1515
revs['Orba Tech'][1] = 1515
# Q3 shortfall relative to Myra Global was 2146 = 2217 - 2146 = 71
revs['Orba Tech'][2] = 71
# Q4 shortfall relative to Myra Global was 282 = 921 - 282 = 639
revs['Orba Tech'][3] = 639

# Prim Dynamics
# Q3 Prim Dynamics earning 2791 more than Orba Tech = 71 + 2791 = 2862
revs['Prim Dynamics'][2] = 2862
# Q4 Prim Dynamics outpaced Orba Tech by 4028 = 639 + 4028 = 4667
revs['Prim Dynamics'][3] = 4667

# Onyx Hub
# Q1 sat 439 under Flux Data = 2165 - 439 = 1726
revs['Onyx Hub'][0] = 1726
# Q2 Flux Data outearned Onyx Hub by 1154 = 3084 - 1154 = 1930
revs['Onyx Hub'][1] = 1930
# Q3 outpaced Prim Dynamics by 599 = 2862 + 599 = 3461
revs['Onyx Hub'][2] = 3461
# Q4 earned 2743 less than Prim Dynamics = 4667 - 2743 = 1924
revs['Onyx Hub'][3] = 1924

# Nero Synth
# Q1 difference came to 1474 in Nero Synth's favor = 1726 + 1474 = 3200
revs['Nero Synth'][0] = 3200
# Q2 surpassed Onyx Hub by 1384 = 1930 + 1384 = 3314
revs['Nero Synth'][1] = 3314
# Q3 shortfall relative to Prim Dynamics was 2025 = 2862 - 2025 = 837
revs['Nero Synth'][2] = 837
# Q4 shortfall relative to Prim Dynamics was 1091 = 4667 - 1091 = 3576
revs['Nero Synth'][3] = 3576

# Tala Pulse
# Q3 lagged Nero Synth by 709 = 837 - 709 = 128
revs['Tala Pulse'][2] = 128
# Q4 beat Nero Synth by 59 = 3576 + 59 = 3635
revs['Tala Pulse'][3] = 3635
# Q1 shortfall relative to Myra Arc was 457 = 3702 - 457 = 3245
revs['Tala Pulse'][0] = 3245
# Q2 gap placed Tala Pulse 4148 behind Myra Arc = 4549 - 4148 = 401
revs['Tala Pulse'][1] = 401

# Tala Global
# Q3 ran 3047 above Tala Pulse = 128 + 3047 = 3175
revs['Tala Global'][2] = 3175
# Q4 led Tala Pulse by 861 = 3635 + 861 = 4496
revs['Tala Global'][3] = 4496

# Fuse Flow
# Q3 ran 179 above Onyx Hub = 3461 + 179 = 3640
revs['Fuse Flow'][2] = 3640
# Q4 difference came to 2288 in Fuse Flow's favor = 1924 + 2288 = 4212
revs['Fuse Flow'][3] = 4212
# Q1 fell 2335 short of Tala Pulse = 3245 - 2335 = 910
revs['Fuse Flow'][0] = 910
# Q2 exceeded Tala Pulse by 2933 = 401 + 2933 = 3334
revs['Fuse Flow'][1] = 3334

# Tala Global Q1
# Q1 posted 597 more than Fuse Flow = 910 + 597 = 1507
revs['Tala Global'][0] = 1507
# Q2 came to 1306 in Tala Global's favor = 3334 + 1306 = 4640
revs['Tala Global'][1] = 4640

# Employees
emps['Tala Prime'] = 55079
emps['Myra Arc'] = 12201
emps['Flux Data'] = 30116
emps['Myra Global'] = 32987
emps['Opti Materials'] = 31445

# Nova Synth maintains 24967 fewer than Tala Prime = 55079 - 24967 = 30112
emps['Nova Synth'] = 30112

# Prim Edge has 20659 more than Nova Synth = 30112 + 20659 = 50771
emps['Prim Edge'] = 50771

# Rune Flow outpaces Nova Synth by 52404 = 30112 + 52404 = 82516
emps['Rune Flow'] = 82516

# Arlo Dynamics employs 40397 more than Myra Arc = 12201 + 40397 = 52598
emps['Arlo Dynamics'] = 52598

# Axio Bio is 6369 fewer than Opti Materials = 31445 - 6369 = 25076
emps['Axio Bio'] = 25076

# Dyna Forge stretches 30815 beyond Flux Data = 30116 + 30815 = 60931
emps['Dyna Forge'] = 60931

# Sero Works retains 49761 additional staff across all divisions = 30116 + 49761 = 79877
emps['Sero Works'] = 79877

# Orba Tech ahead of Flux Data by 383 = 30116 + 383 = 30499
emps['Orba Tech'] = 30499

# Onyx Hub is 21116 positions short of Flux Data = 30116 - 21116 = 9000
emps['Onyx Hub'] = 9000

# Nero Synth fields a team 65890 stronger than Onyx Hub = 9000 + 65890 = 74890
emps['Nero Synth'] = 74890

# Tala Pulse retains 68643 additional staff compared to Myra Arc = 12201 + 68643 = 80844
emps['Tala Pulse'] = 80844

# Fuse Flow is 29450 behind Tala Pulse = 80844 - 29450 = 51394
emps['Fuse Flow'] = 51394

# Tala Global is 42574 fewer than Fuse Flow = 51394 - 42574 = 8820
emps['Tala Global'] = 8820

# Prim Dynamics operates with 78956 more than Myra Arc = 12201 + 78956 = 91157
emps['Prim Dynamics'] = 91157

# Myra Sys 86300 below Prim Dynamics = 91157 - 86300 = 4857
emps['Myra Sys'] = 4857

ratios = {}
for c in data['companies']:
    if None not in revs[c] and emps.get(c) is not None:
        total_rev = sum(revs[c])
        if total_rev > 0:
            ratios[c] = emps[c] / total_rev

for c in sorted(ratios, key=ratios.get, reverse=True):
    print(f"{c}: Emp={emps[c]}, Rev={sum(revs[c])}, Ratio={ratios[c]}")
