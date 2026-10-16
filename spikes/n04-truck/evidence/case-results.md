| Case | Analytic mm3 | Debug outcomes (5 runs) | Release outcomes (5 runs) | Release baseline volume / relative error | B-rep V/E/F; chi; closed | Mesh V/T; missing faces |
|---|---:|---|---|---|---|---|
| plate | 80000 | PASS×3 | PASS×5 | 80000 / 0 | 8/12/6; 2; true | 24/12; 0 |
| g03-unit | 1 | PASS×3 | PASS×5 | 1 / 0 | 8/12/6; 2; true | 24/12; 0 |
| g03-unequal | 3243 | PASS×3 | PASS×5 | 3243 / 0 | 8/12/6; 2; true | 24/12; 0 |
| g03-dyadic | 0.001953125 | PASS×3 | PASS×5 | 0.001953125 / 0 | 8/12/6; 2; true | 24/12; 0 |
| g03-decimal | 0.006 | PASS×3 | PASS×5 | 0.006 / 1.4456e-16 | 8/12/6; 2; true | 24/12; 0 |
| thin-slab | 8 | PASS×3 | PASS×5 | 8 / 0 | 8/12/6; 2; true | 24/12; 0 |
| minimum-cube | 1e-09 | PASS×3 | PASS×5 | 1e-09 / 4.1359e-16 | 8/12/6; 2; true | 24/12; 0 |
| scaled-minimum | 15360 | PASS×3 | PASS×5 | 15360 / 0 | 8/12/6; 2; true | 24/12; 0 |
| scaled-below | 12582.912 | PASS×3 | PASS×5 | 12582.912 / 0 | 8/12/6; 2; true | 24/12; 0 |
| large-scale | 14305114746.1 | PASS×3 | PASS×5 | 14305114746.1 / 0 | 8/12/6; 2; true | 24/12; 0 |
| large-offset | 80000 | PASS×3 | PASS×5 | 80000 / 0 | 8/12/6; 2; true | 24/12; 0 |
| large-offset-thin | 8 | PASS×3 | PASS×5 | 8.00000037998 / 4.7497e-08 | 8/12/6; 2; true | 24/12; 0 |
| hole | 79214.6018366 | FAIL×3 | WRONG×5 | 79215.4372725 / 1.0546e-05 | 14/21/9; 0; true | 3121/5618; 0 |
| pattern | 76858.4073464 | FAIL×3 | FAIL×5 | no mesh | — | — |
| touching-hole | 79214.6018366 | FAIL×3 | PANIC×5 | no mesh | — | — |
| near-hole | 79214.6018366 | FAIL×3 | WRONG×5 | 79215.4378422 / 1.0554e-05 | 14/21/9; 0; true | 3071/5568; 0 |
| near-hole-outside-band | 79214.6018366 | FAIL×3 | WRONG×5 | 79215.4381972 / 1.0558e-05 | 14/21/9; 0; true | 3059/5556; 0 |
| crossing-hole | 79607.3009183 | FAIL×3 | WRONG×5 | 79607.9147683 / 7.711e-06 | 14/21/9; 2; true | 1132/1564; 0 |
| thin-wall | 3.59996 | WRONG×3 | WRONG×5 | 3.62495936277 / 0.0069443 | 16/24/10; 0; true | 228/200; 0 |
| contour | 45000 | PASS×3 | PASS×5 | 45000 / 0 | 12/18/8; 2; true | 36/20; 0 |
| plate-cut-0 | 40000 | PASS×3 | PASS×5 | 40000 / 0 | 8/12/6; 2; true | 108/96; 0 |
| plate-cut-30 | 40000 | PASS×3 | PASS×5 | 40000 / 1.6371e-15 | 8/12/6; 2; true | 114/102; 0 |
| plate-cut-45 | 40000 | PASS×3 | PASS×5 | 40000 / 5.457e-16 | 8/12/6; 2; true | 120/108; 0 |
| plate-cut-60 | 40000 | PASS×3 | PASS×5 | 40000 / 2.9104e-15 | 8/12/6; 2; true | 120/108; 0 |
| plate-cut-vertex | 166.666666667 | PASS×3 | PASS×5 | 166.666666667 / 5.6275e-15 | 4/6/4; 2; true | 18/10; 0 |
| plate-cut-coincident | 80000 | FAIL×3 | FAIL×5 | no mesh | — | — |
| box-cut-0 | 500 | PASS×3 | PASS×5 | 500 / 0 | 8/12/6; 2; true | 24/12; 0 |
| box-cut-30 | 500 | PASS×3 | PASS×5 | 500 / 4.5475e-16 | 8/12/6; 2; true | 80/68; 0 |
| box-cut-45 | 500 | FAIL×3 | FAIL×5 | no mesh | — | — |
| box-cut-60 | 500 | PASS×3 | PASS×5 | 500 / 1.5689e-14 | 8/12/6; 2; true | 80/68; 0 |
| box-cut-vertex | 166.666666667 | PANIC×3 | PANIC×5 | no mesh | — | — |
| box-cut-coincident | 1000 | FAIL×3 | FAIL×5 | no mesh | — | — |
| hole-tol-small | 79214.6018366 | FAIL×3 | FAIL×5 | no mesh | — | — |
| hole-tol-large | 79214.6018366 | FAIL×3 | WRONG×5 | 79215.4310149 / 1.0467e-05 | 14/21/9; 0; true | 3819/6316; 0 |
| hole-tol-scaled | 79214.6018366 | FAIL×3 | WRONG×5 | 79215.4372725 / 1.0546e-05 | 14/21/9; 0; true | 3121/5618; 0 |
| hole-fine-mesh | 79214.6018366 | FAIL×2 | WRONG×5 | 81110.8291191 / 0.023938 | 14/21/9; 0; true | 139660/269300; 1 |
| tess-tol-below | 80000 | PANIC×2 | PANIC×5 | no mesh | — | — |
| tess-tol-equal | 80000 | PASS×2 | PASS×5 | 80000 / 0 | 8/12/6; 2; true | 24/12; 0 |
| nonunit-axis | 79214.6018366 | PANIC×2 | FAIL×5 | no mesh | — | — |
| plate-cut-consistency | 40000 | PANIC×2 | PANIC×5 | no mesh | — | — |
| hole-consistency | 79214.6018366 | PANIC×2 | PANIC×5 | no mesh | — | — |
| hole-coarse | 79214.6018366 | WRONG×2 | WRONG×5 | 79215.4420061 / 1.0606e-05 | 14/21/9; 0; true | 3309/5806; 0 |
| hole-medium-mesh | 79214.6018366 | FAIL×2 | WRONG×5 | 79214.7425902 / 1.7769e-06 | 14/21/9; 0; true | 17473/29362; 0 |
| hole-robust-mesh | 79214.6018366 | FAIL×2 | WRONG×5 | 79214.7425907 / 1.7769e-06 | 14/21/9; 0; true | 17473/29362; 0 |
