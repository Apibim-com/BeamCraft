"""Independent analytic expectations, never derived from truck outputs.

Formulas and citations are derived in the N04 report. G03 uses exact Fraction
references from our repository. No upstream geometry implementation is imported.
"""

from fractions import Fraction
from math import pi, sqrt
from pathlib import Path


def expected(name):
    dims = {"g03-unit": (1.,1.,1.), "g03-unequal": (23.,47.,3.),
        "g03-dyadic": (.125,.25,.0625), "g03-decimal": (.1,.2,.3),
        "thin-slab": (100.,80.,.001), "minimum-cube": (.001,.001,.001),
        "scaled-minimum": (3072.,4096.,5/4096), "scaled-below": (3072.,4096.,.001),
        "large-scale": (300000.,400000.,500000/4194304),
        "large-offset-thin": (100.,80.,.001)}.get(name,(100.,80.,10.))
    if name.startswith("box-cut"):
        dims = (10.,10.,10.)
    a,b,c = dims
    volume = float(Fraction(a)*Fraction(b)*Fraction(c))
    chi = 2
    source = "box-product;cartesian-bounds;euler"
    if "hole" in name or name in ("pattern","nonunit-axis"):
        count = 4 if name == "pattern" else .5 if name == "crossing-hole" else 1
        volume -= count*pi*25*c
        chi = 2 if count == .5 else 2-2*count
        source = "cylinder-integral;genus"
    if "cut" in name:
        if name.endswith("vertex"):
            volume = 1000/6
            source = "tetrahedron-determinant"
        elif not name.endswith("coincident"):
            volume /= 2
            source = "central-inversion-symmetry"
        else:
            source = "closed-halfspace-inclusion"
    if name == "thin-wall":
        # Exact ideal wall thickness 0.001 mm; subtraction in finite precision
        # in the Rust case has a negligible (<1e-10 relative) oracle difference.
        volume = float((Fraction(100)*80-Fraction("99.998")*Fraction("79.998"))*10)
        chi = 0
        source = "difference-of-box-products;genus"
    if name == "contour":
        volume = 45000.
        source = "disjoint-rectangle-area-times-height;euler"
    floor = max(.001,sqrt(a*a+b*b)/4194304)
    return {"case":name,"dimensions_mm":dims,"analytic_volume_mm3":volume,"expected_chi":chi,
        "eps_geom_mm":floor,"f06_dimension_admissible":c>=floor,"formula":source,
        "singular_contact":name=="touching-hole",
        "invalid_upstream_precondition":name=="nonunit-axis"}


def verify_g03():
    path = Path(__file__).resolve().parents[2] / "docs/fixtures/g03/expectations.tsv"
    cases = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("#"):
            continue
        fields = line.split("\t")
        cases[fields[0]] = fields
    for name in ("unit","unequal","dyadic","decimal","all-minimum","scaled-minimum"):
        n04 = {"all-minimum":"minimum-cube"}.get(name,"g03-"+name if name not in ("scaled-minimum",) else name)
        reference = float(Fraction(cases["g03.plate."+name][3]))
        assert expected(n04)["analytic_volume_mm3"] == reference
    return 6
