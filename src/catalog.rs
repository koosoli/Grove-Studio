//! Generated: the CWA-style species and their forest groups, built from the names of the CWA
//! forest models. Each species is written in Solstice's species form. Do not edit by hand.

/// Every species beyond Solstice's own four: id, label, forest group, species text.
pub const CUSTOM: [(&str, &str, &str, &str); 94] = [
    ("solstice-poseidon-oak", "Poseidon oak", "Poseidon oak", include_str!("../species/poseidon-oak.species")),
    ("solstice-cwa-alp-borovice", "Pine", "Pine", r#"# Pine: CWA model tree_afstr_borovice. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.260 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.8 0.3
bark pine_bark
level per_metre=1.8 span=0.45..0.97 angle=70..35 length=0.5 taper=0.3 radius=0.5 upturn=0.3 droop=0 gnarl=0.25 whorl=0
level per_metre=3.5 span=0.2..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.4 whorl=0
leaves fir_needles per_metre=70 length=0.1..0.16 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-borovice-horska", "Pine mountain", "Pine", r#"# Pine mountain: CWA model tree_afstr_borovice_horska. The family sets the shape; the name's variant words adjust it.
height 11.20
trunk radius=0.247 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.8 0.3
bark pine_bark
level per_metre=1.8 span=0.45..0.97 angle=70..35 length=0.5 taper=0.3 radius=0.5 upturn=0.3 droop=0 gnarl=0.25 whorl=0
level per_metre=3.5 span=0.2..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.4 whorl=0
leaves fir_needles per_metre=56 length=0.1..0.16 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-borovice-horska-v1", "Pine mountain v1", "Pine", r#"# Pine mountain v1: CWA model tree_afstr_borovice_horska_v1. The family sets the shape; the name's variant words adjust it.
height 11.20
trunk radius=0.247 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.8 0.3
bark pine_bark
level per_metre=1.8 span=0.45..0.97 angle=70..35 length=0.5 taper=0.3 radius=0.5 upturn=0.3 droop=0 gnarl=0.25 whorl=0
level per_metre=3.5 span=0.2..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.4 whorl=0
leaves fir_needles per_metre=56 length=0.1..0.16 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-borovice-v1", "Pine v1", "Pine", r#"# Pine v1: CWA model tree_afstr_borovice_v1. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.260 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.8 0.3
bark pine_bark
level per_metre=1.8 span=0.45..0.97 angle=70..35 length=0.5 taper=0.3 radius=0.5 upturn=0.3 droop=0 gnarl=0.25 whorl=0
level per_metre=3.5 span=0.2..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.4 whorl=0
leaves fir_needles per_metre=70 length=0.1..0.16 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-jedle", "Fir", "Fir", r#"# Fir: CWA model tree_afstr_jedle. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.220 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=6 span=0.1..0.99 angle=88..65 length=0.3 taper=0.3 radius=0.4 upturn=0 droop=0.2 gnarl=0.05 whorl=5
level per_metre=7 span=0.05..0.97 angle=58..58 length=0.33 taper=0.3 radius=0.45 upturn=0 droop=0.12 gnarl=0.18 whorl=0
leaves fir_needles per_metre=18 length=0.14..0.24 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-jedle-v1", "Fir v1", "Fir", r#"# Fir v1: CWA model tree_afstr_jedle_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.220 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=6 span=0.1..0.99 angle=88..65 length=0.3 taper=0.3 radius=0.4 upturn=0 droop=0.2 gnarl=0.05 whorl=5
level per_metre=7 span=0.05..0.97 angle=58..58 length=0.33 taper=0.3 radius=0.45 upturn=0 droop=0.12 gnarl=0.18 whorl=0
leaves fir_needles per_metre=18 length=0.14..0.24 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-smrk", "Spruce", "Spruce", r#"# Spruce: CWA model tree_afstr_smrk. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-smrk-ridky", "Spruce sparse", "Spruce", r#"# Spruce sparse: CWA model tree_afstr_smrk_ridky. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=4.24 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-smrk-ridky-v1", "Spruce sparse v1", "Spruce", r#"# Spruce sparse v1: CWA model tree_afstr_smrk_ridky_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=4.24 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-alp-smrk-v1", "Spruce v1", "Spruce", r#"# Spruce v1: CWA model tree_afstr_smrk_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-pl-hrusen", "Pear", "Pear", r#"# Pear: CWA model tree_hrusen. The family sets the shape; the name's variant words adjust it.
height 10.00
trunk radius=0.200 flare=0.3 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.3..0.97 angle=45..30 length=0.4 taper=0.3 radius=0.5 upturn=0.25 droop=0 gnarl=0.2 whorl=0
level per_metre=3 span=0.2..0.97 angle=38..38 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=8 span=0.2..1 angle=35..35 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.07..0.1 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-pl-hrusen2", "Pear II", "Pear", r#"# Pear II: CWA model tree_hrusen2. The family sets the shape; the name's variant words adjust it.
height 10.50
trunk radius=0.200 flare=0.3 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.3..0.97 angle=45..30 length=0.4 taper=0.3 radius=0.5 upturn=0.25 droop=0 gnarl=0.2 whorl=0
level per_metre=3 span=0.2..0.97 angle=38..38 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=8 span=0.2..1 angle=35..35 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.07..0.1 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-pl-hrusen2-v1", "Pear II v1", "Pear", r#"# Pear II v1: CWA model tree_hrusen2_v1. The family sets the shape; the name's variant words adjust it.
height 10.50
trunk radius=0.200 flare=0.3 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.3..0.97 angle=45..30 length=0.4 taper=0.3 radius=0.5 upturn=0.25 droop=0 gnarl=0.2 whorl=0
level per_metre=3 span=0.2..0.97 angle=38..38 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=8 span=0.2..1 angle=35..35 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.07..0.1 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-pl-hrusen-v1", "Pear v1", "Pear", r#"# Pear v1: CWA model tree_hrusen_v1. The family sets the shape; the name's variant words adjust it.
height 10.00
trunk radius=0.200 flare=0.3 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.3..0.97 angle=45..30 length=0.4 taper=0.3 radius=0.5 upturn=0.25 droop=0 gnarl=0.2 whorl=0
level per_metre=3 span=0.2..0.97 angle=38..38 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=8 span=0.2..1 angle=35..35 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.07..0.1 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-pl-jablon", "Apple", "Apple", r#"# Apple: CWA model tree_jablon. The family sets the shape; the name's variant words adjust it.
height 8.00
trunk radius=0.200 flare=0.3 gnarl=0.05
crown round
bark oak_bark
level per_metre=3 span=0.3..0.97 angle=60..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.95 angle=48..48 length=0.38 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=8 span=0.2..1 angle=40..40 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.07..0.1 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-pl-jablon-v1", "Apple v1", "Apple", r#"# Apple v1: CWA model tree_jablon_v1. The family sets the shape; the name's variant words adjust it.
height 8.00
trunk radius=0.200 flare=0.3 gnarl=0.05
crown round
bark oak_bark
level per_metre=3 span=0.3..0.97 angle=60..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.95 angle=48..48 length=0.38 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=8 span=0.2..1 angle=40..40 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.07..0.1 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-borovice", "Pine", "Pine", r#"# Pine: CWA model tree_str_borovice. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.260 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.8 0.3
bark pine_bark
level per_metre=1.8 span=0.45..0.97 angle=70..35 length=0.5 taper=0.3 radius=0.5 upturn=0.3 droop=0 gnarl=0.25 whorl=0
level per_metre=3.5 span=0.2..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.4 whorl=0
leaves fir_needles per_metre=70 length=0.1..0.16 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-borovice-horska", "Pine mountain", "Pine", r#"# Pine mountain: CWA model tree_str_borovice_horska. The family sets the shape; the name's variant words adjust it.
height 11.20
trunk radius=0.247 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.8 0.3
bark pine_bark
level per_metre=1.8 span=0.45..0.97 angle=70..35 length=0.5 taper=0.3 radius=0.5 upturn=0.3 droop=0 gnarl=0.25 whorl=0
level per_metre=3.5 span=0.2..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.4 whorl=0
leaves fir_needles per_metre=56 length=0.1..0.16 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-borovice-horska-v1", "Pine mountain v1", "Pine", r#"# Pine mountain v1: CWA model tree_str_borovice_horska_v1. The family sets the shape; the name's variant words adjust it.
height 11.20
trunk radius=0.247 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.8 0.3
bark pine_bark
level per_metre=1.8 span=0.45..0.97 angle=70..35 length=0.5 taper=0.3 radius=0.5 upturn=0.3 droop=0 gnarl=0.25 whorl=0
level per_metre=3.5 span=0.2..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.4 whorl=0
leaves fir_needles per_metre=56 length=0.1..0.16 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-borovice-v1", "Pine v1", "Pine", r#"# Pine v1: CWA model tree_str_borovice_v1. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.260 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.8 0.3
bark pine_bark
level per_metre=1.8 span=0.45..0.97 angle=70..35 length=0.5 taper=0.3 radius=0.5 upturn=0.3 droop=0 gnarl=0.25 whorl=0
level per_metre=3.5 span=0.2..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.4 whorl=0
leaves fir_needles per_metre=70 length=0.1..0.16 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-briza", "Birch", "Birch", r#"# Birch: CWA model tree_str_briza. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.160 flare=0.25 gnarl=0.05
crown round
bark birch_bark
level per_metre=3 span=0.35..0.98 angle=60..30 length=0.36 taper=0.3 radius=0.5 upturn=0.2 droop=0.05 gnarl=0.45 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.5 taper=0.3 radius=0.5 upturn=0 droop=0.4 gnarl=0.45 whorl=0
level per_metre=6 span=0.2..1 angle=30..30 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.5 whorl=0
leaves birch_leaves per_metre=32 length=0.045..0.07 width=0.7 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-briza-kriva", "Birch crooked", "Birch", r#"# Birch crooked: CWA model tree_str_briza_kriva. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.144 flare=0.25 gnarl=0.05
crown round
bark birch_bark
level per_metre=3 span=0.35..0.98 angle=52..22 length=0.36 taper=0.3 radius=0.5 upturn=0.2 droop=0.05 gnarl=0.65 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.5 taper=0.3 radius=0.5 upturn=0 droop=0.4 gnarl=0.45 whorl=0
level per_metre=6 span=0.2..1 angle=30..30 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.5 whorl=0
leaves birch_leaves per_metre=32 length=0.045..0.07 width=0.7 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-briza-kriva-v1", "Birch crooked v1", "Birch", r#"# Birch crooked v1: CWA model tree_str_briza_kriva_v1. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.144 flare=0.25 gnarl=0.05
crown round
bark birch_bark
level per_metre=3 span=0.35..0.98 angle=52..22 length=0.36 taper=0.3 radius=0.5 upturn=0.2 droop=0.05 gnarl=0.65 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.5 taper=0.3 radius=0.5 upturn=0 droop=0.4 gnarl=0.45 whorl=0
level per_metre=6 span=0.2..1 angle=30..30 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.5 whorl=0
leaves birch_leaves per_metre=32 length=0.045..0.07 width=0.7 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-briza-rovna", "Birch straight", "Birch", r#"# Birch straight: CWA model tree_str_briza_rovna. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.160 flare=0.25 gnarl=0.02
crown round
bark birch_bark
level per_metre=3 span=0.35..0.98 angle=65..30 length=0.36 taper=0.3 radius=0.5 upturn=0.2 droop=0.05 gnarl=0.05 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.5 taper=0.3 radius=0.5 upturn=0 droop=0.4 gnarl=0.45 whorl=0
level per_metre=6 span=0.2..1 angle=30..30 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.5 whorl=0
leaves birch_leaves per_metre=32 length=0.045..0.07 width=0.7 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-briza-rovna-v1", "Birch straight v1", "Birch", r#"# Birch straight v1: CWA model tree_str_briza_rovna_v1. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.160 flare=0.25 gnarl=0.02
crown round
bark birch_bark
level per_metre=3 span=0.35..0.98 angle=65..30 length=0.36 taper=0.3 radius=0.5 upturn=0.2 droop=0.05 gnarl=0.05 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.5 taper=0.3 radius=0.5 upturn=0 droop=0.4 gnarl=0.45 whorl=0
level per_metre=6 span=0.2..1 angle=30..30 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.5 whorl=0
leaves birch_leaves per_metre=32 length=0.045..0.07 width=0.7 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-briza-v1", "Birch v1", "Birch", r#"# Birch v1: CWA model tree_str_briza_v1. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.160 flare=0.25 gnarl=0.05
crown round
bark birch_bark
level per_metre=3 span=0.35..0.98 angle=60..30 length=0.36 taper=0.3 radius=0.5 upturn=0.2 droop=0.05 gnarl=0.45 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.5 taper=0.3 radius=0.5 upturn=0 droop=0.4 gnarl=0.45 whorl=0
level per_metre=6 span=0.2..1 angle=30..30 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.5 whorl=0
leaves birch_leaves per_metre=32 length=0.045..0.07 width=0.7 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-briza-v2", "Birch v2", "Birch", r#"# Birch v2: CWA model tree_str_briza_v2. The family sets the shape; the name's variant words adjust it.
height 15.75
trunk radius=0.160 flare=0.25 gnarl=0.05
crown round
bark birch_bark
level per_metre=3 span=0.35..0.98 angle=60..30 length=0.36 taper=0.3 radius=0.5 upturn=0.2 droop=0.05 gnarl=0.45 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.5 taper=0.3 radius=0.5 upturn=0 droop=0.4 gnarl=0.45 whorl=0
level per_metre=6 span=0.2..1 angle=30..30 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.5 whorl=0
leaves birch_leaves per_metre=32 length=0.045..0.07 width=0.7 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-buk", "Beech", "Beech", r#"# Beech: CWA model tree_str_buk. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.280 flare=0.4 gnarl=0.05
crown round
bark beech_bark
level per_metre=3.4 span=0.3..0.97 angle=50..30 length=0.42 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.2 whorl=0
level per_metre=2.8 span=0.2..1 angle=42..42 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=7 span=0.25..1 angle=35..35 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=24 length=0.08..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-buk-v1", "Beech v1", "Beech", r#"# Beech v1: CWA model tree_str_buk_v1. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.280 flare=0.4 gnarl=0.05
crown round
bark beech_bark
level per_metre=3.4 span=0.3..0.97 angle=50..30 length=0.42 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.2 whorl=0
level per_metre=2.8 span=0.2..1 angle=42..42 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=7 span=0.25..1 angle=35..35 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=24 length=0.08..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-buk-v2", "Beech v2", "Beech", r#"# Beech v2: CWA model tree_str_buk_v2. The family sets the shape; the name's variant words adjust it.
height 15.75
trunk radius=0.280 flare=0.4 gnarl=0.05
crown round
bark beech_bark
level per_metre=3.4 span=0.3..0.97 angle=50..30 length=0.42 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.2 whorl=0
level per_metre=2.8 span=0.2..1 angle=42..42 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=7 span=0.25..1 angle=35..35 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=24 length=0.08..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-dub", "Oak", "Oak", r#"# Oak: CWA model tree_str_dub. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.320 flare=0.6 gnarl=0.05
crown round
bark oak_bark
level per_metre=3.2 span=0.3..0.95 angle=70..45 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.15..0.95 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.4 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.6 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.09..0.14 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-dub-jiny", "Oak other", "Oak", r#"# Oak other: CWA model tree_str_dub_jiny. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.320 flare=0.6 gnarl=0.05
crown round
bark oak_bark
level per_metre=3.2 span=0.3..0.95 angle=78..53 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.15..0.95 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.4 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.6 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.09..0.14 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-dub-jiny-v1", "Oak other v1", "Oak", r#"# Oak other v1: CWA model tree_str_dub_jiny_v1. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.320 flare=0.6 gnarl=0.05
crown round
bark oak_bark
level per_metre=3.2 span=0.3..0.95 angle=78..53 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.15..0.95 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.4 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.6 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.09..0.14 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-dub-jiny-v2", "Oak other v2", "Oak", r#"# Oak other v2: CWA model tree_str_dub_jiny_v2. The family sets the shape; the name's variant words adjust it.
height 14.70
trunk radius=0.320 flare=0.6 gnarl=0.05
crown round
bark oak_bark
level per_metre=3.2 span=0.3..0.95 angle=78..53 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.15..0.95 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.4 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.6 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.09..0.14 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-dub-v1", "Oak v1", "Oak", r#"# Oak v1: CWA model tree_str_dub_v1. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.320 flare=0.6 gnarl=0.05
crown round
bark oak_bark
level per_metre=3.2 span=0.3..0.95 angle=70..45 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.15..0.95 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.4 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.6 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.09..0.14 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-dub-v2", "Oak v2", "Oak", r#"# Oak v2: CWA model tree_str_dub_v2. The family sets the shape; the name's variant words adjust it.
height 14.70
trunk radius=0.320 flare=0.6 gnarl=0.05
crown round
bark oak_bark
level per_metre=3.2 span=0.3..0.95 angle=70..45 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.15..0.95 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.4 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.6 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.09..0.14 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-fikovnik", "Fig", "Fig", r#"# Fig: CWA model tree_str_fikovnik. The family sets the shape; the name's variant words adjust it.
height 7.00
trunk radius=0.160 flare=0.3 gnarl=0.05
crown even
bark oak_bark
level per_metre=3 span=0.1..0.97 angle=60..30 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.1..0.16 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-fikovnik2", "Fig II", "Fig", r#"# Fig II: CWA model tree_str_fikovnik2. The family sets the shape; the name's variant words adjust it.
height 7.35
trunk radius=0.160 flare=0.3 gnarl=0.05
crown even
bark oak_bark
level per_metre=3 span=0.1..0.97 angle=60..30 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.1..0.16 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-fikovnik2-v1", "Fig II v1", "Fig", r#"# Fig II v1: CWA model tree_str_fikovnik2_v1. The family sets the shape; the name's variant words adjust it.
height 7.35
trunk radius=0.160 flare=0.3 gnarl=0.05
crown even
bark oak_bark
level per_metre=3 span=0.1..0.97 angle=60..30 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.1..0.16 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-fikovnik-ker", "Fig shrub", "Fig", r#"# Fig shrub: CWA model tree_str_fikovnik_ker. The family sets the shape; the name's variant words adjust it.
height 4.20
trunk radius=0.160 flare=0.3 gnarl=0.05
crown even
bark oak_bark
level per_metre=4.5 span=0.1..0.97 angle=60..30 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.1..0.16 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-fikovnik-ker-v1", "Fig shrub v1", "Fig", r#"# Fig shrub v1: CWA model tree_str_fikovnik_ker_v1. The family sets the shape; the name's variant words adjust it.
height 4.20
trunk radius=0.160 flare=0.3 gnarl=0.05
crown even
bark oak_bark
level per_metre=4.5 span=0.1..0.97 angle=60..30 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.1..0.16 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-fikovnik-v1", "Fig v1", "Fig", r#"# Fig v1: CWA model tree_str_fikovnik_v1. The family sets the shape; the name's variant words adjust it.
height 7.00
trunk radius=0.160 flare=0.3 gnarl=0.05
crown even
bark oak_bark
level per_metre=3 span=0.1..0.97 angle=60..30 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.1..0.16 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-habr", "Hornbeam", "Hornbeam", r#"# Hornbeam: CWA model tree_str_habr. The family sets the shape; the name's variant words adjust it.
height 12.00
trunk radius=0.220 flare=0.4 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.3..0.97 angle=60..38 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=2.8 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=7 span=0.2..1 angle=38..38 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.07..0.11 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-habr-v1", "Hornbeam v1", "Hornbeam", r#"# Hornbeam v1: CWA model tree_str_habr_v1. The family sets the shape; the name's variant words adjust it.
height 12.00
trunk radius=0.220 flare=0.4 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.3..0.97 angle=60..38 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=2.8 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=7 span=0.2..1 angle=38..38 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.07..0.11 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-habr-v2", "Hornbeam v2", "Hornbeam", r#"# Hornbeam v2: CWA model tree_str_habr_v2. The family sets the shape; the name's variant words adjust it.
height 12.60
trunk radius=0.220 flare=0.4 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.3..0.97 angle=60..38 length=0.42 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=2.8 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=7 span=0.2..1 angle=38..38 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.07..0.11 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-jablon", "Apple", "Apple", r#"# Apple: CWA model tree_str_jablon. The family sets the shape; the name's variant words adjust it.
height 8.00
trunk radius=0.200 flare=0.3 gnarl=0.05
crown round
bark oak_bark
level per_metre=3 span=0.3..0.97 angle=60..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.95 angle=48..48 length=0.38 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=8 span=0.2..1 angle=40..40 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.07..0.1 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-jablon-v1", "Apple v1", "Apple", r#"# Apple v1: CWA model tree_str_jablon_v1. The family sets the shape; the name's variant words adjust it.
height 8.00
trunk radius=0.200 flare=0.3 gnarl=0.05
crown round
bark oak_bark
level per_metre=3 span=0.3..0.97 angle=60..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.95 angle=48..48 length=0.38 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.35 whorl=0
level per_metre=8 span=0.2..1 angle=40..40 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.07..0.1 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-jalovec", "Juniper", "Juniper", r#"# Juniper: CWA model tree_str_jalovec. The family sets the shape; the name's variant words adjust it.
height 4.00
trunk radius=0.100 flare=0.2 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.05..0.99 angle=70..40 length=0.28 taper=0.3 radius=0.5 upturn=0.1 droop=0.1 gnarl=0.15 whorl=4
level per_metre=6 span=0.05..0.97 angle=50..50 length=0.3 taper=0.3 radius=0.5 upturn=0 droop=0 gnarl=0.25 whorl=0
leaves fir_needles per_metre=40 length=0.12..0.2 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-jalovec-v1", "Juniper v1", "Juniper", r#"# Juniper v1: CWA model tree_str_jalovec_v1. The family sets the shape; the name's variant words adjust it.
height 4.00
trunk radius=0.100 flare=0.2 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.05..0.99 angle=70..40 length=0.28 taper=0.3 radius=0.5 upturn=0.1 droop=0.1 gnarl=0.15 whorl=4
level per_metre=6 span=0.05..0.97 angle=50..50 length=0.3 taper=0.3 radius=0.5 upturn=0 droop=0 gnarl=0.25 whorl=0
leaves fir_needles per_metre=40 length=0.12..0.2 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-javor", "Maple", "Maple", r#"# Maple: CWA model tree_str_javor. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.260 flare=0.35 gnarl=0.05
crown round
bark oak_bark
level per_metre=3 span=0.3..0.97 angle=62..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.2 whorl=0
level per_metre=2.6 span=0.2..0.97 angle=48..48 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.07..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-javor-v1", "Maple v1", "Maple", r#"# Maple v1: CWA model tree_str_javor_v1. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.260 flare=0.35 gnarl=0.05
crown round
bark oak_bark
level per_metre=3 span=0.3..0.97 angle=62..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.2 whorl=0
level per_metre=2.6 span=0.2..0.97 angle=48..48 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.07..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-javor-v2", "Maple v2", "Maple", r#"# Maple v2: CWA model tree_str_javor_v2. The family sets the shape; the name's variant words adjust it.
height 14.70
trunk radius=0.260 flare=0.35 gnarl=0.05
crown round
bark oak_bark
level per_metre=3 span=0.3..0.97 angle=62..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.2 whorl=0
level per_metre=2.6 span=0.2..0.97 angle=48..48 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.07..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-jedle", "Fir", "Fir", r#"# Fir: CWA model tree_str_jedle. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.220 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=6 span=0.1..0.99 angle=88..65 length=0.3 taper=0.3 radius=0.4 upturn=0 droop=0.2 gnarl=0.05 whorl=5
level per_metre=7 span=0.05..0.97 angle=58..58 length=0.33 taper=0.3 radius=0.45 upturn=0 droop=0.12 gnarl=0.18 whorl=0
leaves fir_needles per_metre=18 length=0.14..0.24 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-jedle-v1", "Fir v1", "Fir", r#"# Fir v1: CWA model tree_str_jedle_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.220 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=6 span=0.1..0.99 angle=88..65 length=0.3 taper=0.3 radius=0.4 upturn=0 droop=0.2 gnarl=0.05 whorl=5
level per_metre=7 span=0.05..0.97 angle=58..58 length=0.33 taper=0.3 radius=0.45 upturn=0 droop=0.12 gnarl=0.18 whorl=0
leaves fir_needles per_metre=18 length=0.14..0.24 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-jerabina", "Rowan", "Rowan", r#"# Rowan: CWA model tree_str_jerabina. The family sets the shape; the name's variant words adjust it.
height 9.00
trunk radius=0.160 flare=0.2 gnarl=0.05
crown round
bark beech_bark
level per_metre=3.5 span=0.25..0.97 angle=60..35 length=0.35 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.38 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.35 whorl=0
level per_metre=7 span=0.2..1 angle=38..38 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.06..0.09 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-jerabina-v1", "Rowan v1", "Rowan", r#"# Rowan v1: CWA model tree_str_jerabina_v1. The family sets the shape; the name's variant words adjust it.
height 9.00
trunk radius=0.160 flare=0.2 gnarl=0.05
crown round
bark beech_bark
level per_metre=3.5 span=0.25..0.97 angle=60..35 length=0.35 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.3 whorl=0
level per_metre=3 span=0.2..0.95 angle=45..45 length=0.38 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.35 whorl=0
level per_metre=7 span=0.2..1 angle=38..38 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.06..0.09 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-kastan", "Chestnut", "Chestnut", r#"# Chestnut: CWA model tree_str_kastan. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.340 flare=0.5 gnarl=0.05
crown round
bark oak_bark
level per_metre=2.6 span=0.3..0.97 angle=62..38 length=0.44 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.2..0.97 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.35 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.36 taper=0.3 radius=0.6 upturn=0 droop=0.25 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=30 length=0.12..0.2 width=0.5 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-kastan-v1", "Chestnut v1", "Chestnut", r#"# Chestnut v1: CWA model tree_str_kastan_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.340 flare=0.5 gnarl=0.05
crown round
bark oak_bark
level per_metre=2.6 span=0.3..0.97 angle=62..38 length=0.44 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.2..0.97 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.35 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.36 taper=0.3 radius=0.6 upturn=0 droop=0.25 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=30 length=0.12..0.2 width=0.5 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-kastan-v2", "Chestnut v2", "Chestnut", r#"# Chestnut v2: CWA model tree_str_kastan_v2. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.340 flare=0.5 gnarl=0.05
crown round
bark oak_bark
level per_metre=2.6 span=0.3..0.97 angle=62..38 length=0.44 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.25 whorl=0
level per_metre=2.4 span=0.2..0.97 angle=45..45 length=0.45 taper=0.3 radius=0.6 upturn=0 droop=0.05 gnarl=0.35 whorl=0
level per_metre=7 span=0.2..1 angle=40..40 length=0.36 taper=0.3 radius=0.6 upturn=0 droop=0.25 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=30 length=0.12..0.2 width=0.5 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-krovisko-vysoke", "Krovisko vysoke", "Krovisko", r#"# Krovisko vysoke: CWA model tree_str_krovisko_vysoke. The family sets the shape; the name's variant words adjust it.
height 6.00
trunk radius=0.140 flare=0.2 gnarl=0.05
crown round
bark beech_bark
level per_metre=4 span=0.2..1 angle=60..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.4 whorl=0
level per_metre=4 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.07..0.11 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-krovisko-vysoke-v1", "Krovisko vysoke v1", "Krovisko", r#"# Krovisko vysoke v1: CWA model tree_str_krovisko_vysoke_v1. The family sets the shape; the name's variant words adjust it.
height 6.00
trunk radius=0.140 flare=0.2 gnarl=0.05
crown round
bark beech_bark
level per_metre=4 span=0.2..1 angle=60..40 length=0.4 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.4 whorl=0
level per_metre=4 span=0.2..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
leaves broadleaf_leaves per_metre=36 length=0.07..0.11 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-lipa", "Linden", "Linden", r#"# Linden: CWA model tree_str_lipa. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.280 flare=0.4 gnarl=0.05
crown round
bark oak_bark
level per_metre=2.8 span=0.3..0.97 angle=55..35 length=0.44 taper=0.3 radius=0.5 upturn=0.15 droop=0 gnarl=0.2 whorl=0
level per_metre=2.6 span=0.2..0.97 angle=45..45 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=6 span=0.2..1 angle=38..38 length=0.36 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=38 length=0.08..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-lipa-v1", "Linden v1", "Linden", r#"# Linden v1: CWA model tree_str_lipa_v1. The family sets the shape; the name's variant words adjust it.
height 15.00
trunk radius=0.280 flare=0.4 gnarl=0.05
crown round
bark oak_bark
level per_metre=2.8 span=0.3..0.97 angle=55..35 length=0.44 taper=0.3 radius=0.5 upturn=0.15 droop=0 gnarl=0.2 whorl=0
level per_metre=2.6 span=0.2..0.97 angle=45..45 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=6 span=0.2..1 angle=38..38 length=0.36 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=38 length=0.08..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-lipa-v2", "Linden v2", "Linden", r#"# Linden v2: CWA model tree_str_lipa_v2. The family sets the shape; the name's variant words adjust it.
height 15.75
trunk radius=0.280 flare=0.4 gnarl=0.05
crown round
bark oak_bark
level per_metre=2.8 span=0.3..0.97 angle=55..35 length=0.44 taper=0.3 radius=0.5 upturn=0.15 droop=0 gnarl=0.2 whorl=0
level per_metre=2.6 span=0.2..0.97 angle=45..45 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.3 whorl=0
level per_metre=6 span=0.2..1 angle=38..38 length=0.36 taper=0.3 radius=0.6 upturn=0 droop=0.2 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=38 length=0.08..0.12 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-liskac", "Hazel", "Hazel", r#"# Hazel: CWA model tree_str_liskac. The family sets the shape; the name's variant words adjust it.
height 4.00
trunk radius=0.080 flare=0.2 gnarl=0.05
crown even
bark oak_bark
level per_metre=6 span=0..1 angle=70..40 length=0.35 taper=0.3 radius=0.6 upturn=0.15 droop=0 gnarl=0.45 whorl=0
level per_metre=6 span=0.1..0.97 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.05..0.08 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-liskac-v1", "Hazel v1", "Hazel", r#"# Hazel v1: CWA model tree_str_liskac_v1. The family sets the shape; the name's variant words adjust it.
height 4.00
trunk radius=0.080 flare=0.2 gnarl=0.05
crown even
bark oak_bark
level per_metre=6 span=0..1 angle=70..40 length=0.35 taper=0.3 radius=0.6 upturn=0.15 droop=0 gnarl=0.45 whorl=0
level per_metre=6 span=0.1..0.97 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.05..0.08 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-osika", "Aspen", "Aspen", r#"# Aspen: CWA model tree_str_osika. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.220 flare=0.3 gnarl=0.05
crown round
bark birch_bark
level per_metre=3 span=0.3..0.98 angle=55..35 length=0.36 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.2 whorl=0
level per_metre=3.5 span=0.2..0.95 angle=45..45 length=0.42 taper=0.3 radius=0.55 upturn=0 droop=0.1 gnarl=0.3 whorl=0
level per_metre=6 span=0.2..1 angle=35..35 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.3 gnarl=0.3 whorl=0
leaves birch_leaves per_metre=38 length=0.04..0.06 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-osika-v1", "Aspen v1", "Aspen", r#"# Aspen v1: CWA model tree_str_osika_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.220 flare=0.3 gnarl=0.05
crown round
bark birch_bark
level per_metre=3 span=0.3..0.98 angle=55..35 length=0.36 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.2 whorl=0
level per_metre=3.5 span=0.2..0.95 angle=45..45 length=0.42 taper=0.3 radius=0.55 upturn=0 droop=0.1 gnarl=0.3 whorl=0
level per_metre=6 span=0.2..1 angle=35..35 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.3 gnarl=0.3 whorl=0
leaves birch_leaves per_metre=38 length=0.04..0.06 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-osika-v2", "Aspen v2", "Aspen", r#"# Aspen v2: CWA model tree_str_osika_v2. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.220 flare=0.3 gnarl=0.05
crown round
bark birch_bark
level per_metre=3 span=0.3..0.98 angle=55..35 length=0.36 taper=0.3 radius=0.5 upturn=0.1 droop=0 gnarl=0.2 whorl=0
level per_metre=3.5 span=0.2..0.95 angle=45..45 length=0.42 taper=0.3 radius=0.55 upturn=0 droop=0.1 gnarl=0.3 whorl=0
level per_metre=6 span=0.2..1 angle=35..35 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.3 gnarl=0.3 whorl=0
leaves birch_leaves per_metre=38 length=0.04..0.06 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-pinie", "Stone pine", "Stone pine", r#"# Stone pine: CWA model tree_str_pinie. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.280 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.9 0.5
bark pine_bark
level per_metre=1.6 span=0.5..0.97 angle=62..40 length=0.5 taper=0.3 radius=0.5 upturn=0.4 droop=0 gnarl=0.2 whorl=0
level per_metre=3 span=0.25..1 angle=45..45 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.35 whorl=0
leaves fir_needles per_metre=60 length=0.12..0.18 width=0.25 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-pinie-v1", "Stone pine v1", "Stone pine", r#"# Stone pine v1: CWA model tree_str_pinie_v1. The family sets the shape; the name's variant words adjust it.
height 14.00
trunk radius=0.280 flare=0.3 gnarl=0.05
crown profile 0.2 0.6 1 0.9 0.5
bark pine_bark
level per_metre=1.6 span=0.5..0.97 angle=62..40 length=0.5 taper=0.3 radius=0.5 upturn=0.4 droop=0 gnarl=0.2 whorl=0
level per_metre=3 span=0.25..1 angle=45..45 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0.1 gnarl=0.35 whorl=0
leaves fir_needles per_metre=60 length=0.12..0.18 width=0.25 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrcicicek", "Dwarf spruce", "Dwarf spruce", r#"# Dwarf spruce: CWA model tree_str_smrcicicek. The family sets the shape; the name's variant words adjust it.
height 4.00
trunk radius=0.120 flare=0.2 gnarl=0.05
crown cone
bark fir_bark
level per_metre=8 span=0.1..0.99 angle=85..60 length=0.35 taper=0.3 radius=0.45 upturn=0 droop=0.2 gnarl=0.1 whorl=4
level per_metre=7 span=0.05..0.97 angle=55..55 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.1 gnarl=0.2 whorl=0
leaves fir_needles per_metre=22 length=0.12..0.2 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrcicicek-les", "Dwarf spruce forest", "Dwarf spruce", r#"# Dwarf spruce forest: CWA model tree_str_smrcicicek_les. The family sets the shape; the name's variant words adjust it.
height 4.60
trunk radius=0.108 flare=0.2 gnarl=0.05
crown cone
bark fir_bark
level per_metre=8 span=0.1..0.99 angle=79..60 length=0.35 taper=0.3 radius=0.45 upturn=0 droop=0.2 gnarl=0.1 whorl=4
level per_metre=7 span=0.05..0.97 angle=55..55 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.1 gnarl=0.2 whorl=0
leaves fir_needles per_metre=22 length=0.12..0.2 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrcicicek-les-v1", "Dwarf spruce forest v1", "Dwarf spruce", r#"# Dwarf spruce forest v1: CWA model tree_str_smrcicicek_les_v1. The family sets the shape; the name's variant words adjust it.
height 4.60
trunk radius=0.108 flare=0.2 gnarl=0.05
crown cone
bark fir_bark
level per_metre=8 span=0.1..0.99 angle=79..60 length=0.35 taper=0.3 radius=0.45 upturn=0 droop=0.2 gnarl=0.1 whorl=4
level per_metre=7 span=0.05..0.97 angle=55..55 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.1 gnarl=0.2 whorl=0
leaves fir_needles per_metre=22 length=0.12..0.2 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrcicicek-v1", "Dwarf spruce v1", "Dwarf spruce", r#"# Dwarf spruce v1: CWA model tree_str_smrcicicek_v1. The family sets the shape; the name's variant words adjust it.
height 4.00
trunk radius=0.120 flare=0.2 gnarl=0.05
crown cone
bark fir_bark
level per_metre=8 span=0.1..0.99 angle=85..60 length=0.35 taper=0.3 radius=0.45 upturn=0 droop=0.2 gnarl=0.1 whorl=4
level per_metre=7 span=0.05..0.97 angle=55..55 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.1 gnarl=0.2 whorl=0
leaves fir_needles per_metre=22 length=0.12..0.2 width=0.2 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk", "Spruce", "Spruce", r#"# Spruce: CWA model tree_str_smrk. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-dvoj", "Spruce forked", "Spruce", r#"# Spruce forked: CWA model tree_str_smrk_dvoj. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.192 flare=0.6000000000000001 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-dvoj-v1", "Spruce forked v1", "Spruce", r#"# Spruce forked v1: CWA model tree_str_smrk_dvoj_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.192 flare=0.6000000000000001 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-medium", "Spruce medium", "Spruce", r#"# Spruce medium: CWA model tree_str_smrk_medium. The family sets the shape; the name's variant words adjust it.
height 13.60
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-medium-v1", "Spruce medium v1", "Spruce", r#"# Spruce medium v1: CWA model tree_str_smrk_medium_v1. The family sets the shape; the name's variant words adjust it.
height 13.60
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-ridky", "Spruce sparse", "Spruce", r#"# Spruce sparse: CWA model tree_str_smrk_ridky. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=4.24 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-ridky-v1", "Spruce sparse v1", "Spruce", r#"# Spruce sparse v1: CWA model tree_str_smrk_ridky_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=4.24 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-v1", "Spruce v1", "Spruce", r#"# Spruce v1: CWA model tree_str_smrk_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-vysoky", "Spruce tall", "Spruce", r#"# Spruce tall: CWA model tree_str_smrk_vysoky. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-smrk-vysoky-v1", "Spruce tall v1", "Spruce", r#"# Spruce tall v1: CWA model tree_str_smrk_vysoky_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.240 flare=0.4 gnarl=0.05
crown cone
bark fir_bark
level per_metre=7 span=0.08..0.99 angle=95..70 length=0.32 taper=0.3 radius=0.42 upturn=0 droop=0.25 gnarl=0.1 whorl=6
level per_metre=6 span=0.05..0.97 angle=60..60 length=0.35 taper=0.3 radius=0.5 upturn=0 droop=0.15 gnarl=0.2 whorl=0
leaves fir_needles per_metre=14 length=0.16..0.26 width=0.17 variants=0.4,0.3,0.2,0.1 normals=0.4
"#),
    ("solstice-cwa-topol", "Poplar", "Poplar", r#"# Poplar: CWA model tree_str_topol. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.260 flare=0.2 gnarl=0.05
crown cone
bark oak_bark
level per_metre=6 span=0.1..0.97 angle=70..45 length=0.3 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.1 whorl=0
level per_metre=5 span=0.1..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.5 upturn=0 droop=0 gnarl=0.15 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.07..0.1 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-topol2", "Poplar II", "Poplar", r#"# Poplar II: CWA model tree_str_topol2. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.260 flare=0.2 gnarl=0.05
crown cone
bark oak_bark
level per_metre=6 span=0.1..0.97 angle=70..45 length=0.3 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.1 whorl=0
level per_metre=5 span=0.1..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.5 upturn=0 droop=0 gnarl=0.15 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.07..0.1 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-topol2-v1", "Poplar II v1", "Poplar", r#"# Poplar II v1: CWA model tree_str_topol2_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.260 flare=0.2 gnarl=0.05
crown cone
bark oak_bark
level per_metre=6 span=0.1..0.97 angle=70..45 length=0.3 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.1 whorl=0
level per_metre=5 span=0.1..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.5 upturn=0 droop=0 gnarl=0.15 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.07..0.1 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-topol-v1", "Poplar v1", "Poplar", r#"# Poplar v1: CWA model tree_str_topol_v1. The family sets the shape; the name's variant words adjust it.
height 16.00
trunk radius=0.260 flare=0.2 gnarl=0.05
crown cone
bark oak_bark
level per_metre=6 span=0.1..0.97 angle=70..45 length=0.3 taper=0.3 radius=0.5 upturn=0.2 droop=0 gnarl=0.1 whorl=0
level per_metre=5 span=0.1..0.97 angle=45..45 length=0.4 taper=0.3 radius=0.5 upturn=0 droop=0 gnarl=0.15 whorl=0
leaves broadleaf_leaves per_metre=34 length=0.07..0.1 width=0.6 variants=0.4,0.3,0.2,0.1 normals=0.45
"#),
    ("solstice-cwa-trnka", "Blackthorn", "Blackthorn", r#"# Blackthorn: CWA model tree_str_trnka. The family sets the shape; the name's variant words adjust it.
height 4.00
trunk radius=0.090 flare=0.2 gnarl=0.05
crown even
bark oak_bark
level per_metre=7 span=0..1 angle=80..50 length=0.35 taper=0.3 radius=0.6 upturn=0.1 droop=0.1 gnarl=0.6 whorl=0
level per_metre=5 span=0.1..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.05..0.08 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-trnka-v1", "Blackthorn v1", "Blackthorn", r#"# Blackthorn v1: CWA model tree_str_trnka_v1. The family sets the shape; the name's variant words adjust it.
height 4.00
trunk radius=0.090 flare=0.2 gnarl=0.05
crown even
bark oak_bark
level per_metre=7 span=0..1 angle=80..50 length=0.35 taper=0.3 radius=0.6 upturn=0.1 droop=0.1 gnarl=0.6 whorl=0
level per_metre=5 span=0.1..1 angle=50..50 length=0.35 taper=0.3 radius=0.6 upturn=0 droop=0 gnarl=0.5 whorl=0
leaves broadleaf_leaves per_metre=40 length=0.05..0.08 width=0.55 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-vrba", "Willow", "Willow", r#"# Willow: CWA model tree_str_vrba. The family sets the shape; the name's variant words adjust it.
height 12.00
trunk radius=0.220 flare=0.3 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.2..0.97 angle=60..40 length=0.4 taper=0.3 radius=0.5 upturn=0 droop=0.35 gnarl=0.3 whorl=0
level per_metre=3.5 span=0.2..0.97 angle=45..45 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0.5 gnarl=0.3 whorl=0
level per_metre=8 span=0.2..1 angle=30..30 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=42 length=0.06..0.12 width=0.45 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
    ("solstice-cwa-vrba-v1", "Willow v1", "Willow", r#"# Willow v1: CWA model tree_str_vrba_v1. The family sets the shape; the name's variant words adjust it.
height 12.00
trunk radius=0.220 flare=0.3 gnarl=0.05
crown round
bark beech_bark
level per_metre=3 span=0.2..0.97 angle=60..40 length=0.4 taper=0.3 radius=0.5 upturn=0 droop=0.35 gnarl=0.3 whorl=0
level per_metre=3.5 span=0.2..0.97 angle=45..45 length=0.42 taper=0.3 radius=0.6 upturn=0 droop=0.5 gnarl=0.3 whorl=0
level per_metre=8 span=0.2..1 angle=30..30 length=0.3 taper=0.3 radius=0.6 upturn=0 droop=0.6 gnarl=0.4 whorl=0
leaves broadleaf_leaves per_metre=42 length=0.06..0.12 width=0.45 variants=0.4,0.3,0.2,0.1 normals=0.5
"#),
];
/// Every species id, Solstice's first.
pub const IDS: [&str; 98] = [
    "solstice-oak", "solstice-birch", "solstice-fir", "solstice-bush",
    "solstice-poseidon-oak", "solstice-cwa-alp-borovice", "solstice-cwa-alp-borovice-horska", "solstice-cwa-alp-borovice-horska-v1",
    "solstice-cwa-alp-borovice-v1", "solstice-cwa-alp-jedle", "solstice-cwa-alp-jedle-v1", "solstice-cwa-alp-smrk",
    "solstice-cwa-alp-smrk-ridky", "solstice-cwa-alp-smrk-ridky-v1", "solstice-cwa-alp-smrk-v1", "solstice-cwa-pl-hrusen",
    "solstice-cwa-pl-hrusen2", "solstice-cwa-pl-hrusen2-v1", "solstice-cwa-pl-hrusen-v1", "solstice-cwa-pl-jablon",
    "solstice-cwa-pl-jablon-v1", "solstice-cwa-borovice", "solstice-cwa-borovice-horska", "solstice-cwa-borovice-horska-v1",
    "solstice-cwa-borovice-v1", "solstice-cwa-briza", "solstice-cwa-briza-kriva", "solstice-cwa-briza-kriva-v1",
    "solstice-cwa-briza-rovna", "solstice-cwa-briza-rovna-v1", "solstice-cwa-briza-v1", "solstice-cwa-briza-v2",
    "solstice-cwa-buk", "solstice-cwa-buk-v1", "solstice-cwa-buk-v2", "solstice-cwa-dub",
    "solstice-cwa-dub-jiny", "solstice-cwa-dub-jiny-v1", "solstice-cwa-dub-jiny-v2", "solstice-cwa-dub-v1",
    "solstice-cwa-dub-v2", "solstice-cwa-fikovnik", "solstice-cwa-fikovnik2", "solstice-cwa-fikovnik2-v1",
    "solstice-cwa-fikovnik-ker", "solstice-cwa-fikovnik-ker-v1", "solstice-cwa-fikovnik-v1", "solstice-cwa-habr",
    "solstice-cwa-habr-v1", "solstice-cwa-habr-v2", "solstice-cwa-jablon", "solstice-cwa-jablon-v1",
    "solstice-cwa-jalovec", "solstice-cwa-jalovec-v1", "solstice-cwa-javor", "solstice-cwa-javor-v1",
    "solstice-cwa-javor-v2", "solstice-cwa-jedle", "solstice-cwa-jedle-v1", "solstice-cwa-jerabina",
    "solstice-cwa-jerabina-v1", "solstice-cwa-kastan", "solstice-cwa-kastan-v1", "solstice-cwa-kastan-v2",
    "solstice-cwa-krovisko-vysoke", "solstice-cwa-krovisko-vysoke-v1", "solstice-cwa-lipa", "solstice-cwa-lipa-v1",
    "solstice-cwa-lipa-v2", "solstice-cwa-liskac", "solstice-cwa-liskac-v1", "solstice-cwa-osika",
    "solstice-cwa-osika-v1", "solstice-cwa-osika-v2", "solstice-cwa-pinie", "solstice-cwa-pinie-v1",
    "solstice-cwa-smrcicicek", "solstice-cwa-smrcicicek-les", "solstice-cwa-smrcicicek-les-v1", "solstice-cwa-smrcicicek-v1",
    "solstice-cwa-smrk", "solstice-cwa-smrk-dvoj", "solstice-cwa-smrk-dvoj-v1", "solstice-cwa-smrk-medium",
    "solstice-cwa-smrk-medium-v1", "solstice-cwa-smrk-ridky", "solstice-cwa-smrk-ridky-v1", "solstice-cwa-smrk-v1",
    "solstice-cwa-smrk-vysoky", "solstice-cwa-smrk-vysoky-v1", "solstice-cwa-topol", "solstice-cwa-topol2",
    "solstice-cwa-topol2-v1", "solstice-cwa-topol-v1", "solstice-cwa-trnka", "solstice-cwa-trnka-v1",
    "solstice-cwa-vrba", "solstice-cwa-vrba-v1",
];
/// Forest groups: a label and the species that share the group's percentage.
pub const GROUPS: [(&str, &[&str]); 28] = [
    ("Solstice oak", &["solstice-oak"]),
    ("Solstice birch", &["solstice-birch"]),
    ("Solstice fir", &["solstice-fir"]),
    ("Solstice bush", &["solstice-bush"]),
    ("Poseidon oak", &["solstice-poseidon-oak"]),
    ("Pine", &["solstice-cwa-alp-borovice", "solstice-cwa-alp-borovice-horska", "solstice-cwa-alp-borovice-horska-v1", "solstice-cwa-alp-borovice-v1", "solstice-cwa-borovice", "solstice-cwa-borovice-horska", "solstice-cwa-borovice-horska-v1", "solstice-cwa-borovice-v1"]),
    ("Fir", &["solstice-cwa-alp-jedle", "solstice-cwa-alp-jedle-v1", "solstice-cwa-jedle", "solstice-cwa-jedle-v1"]),
    ("Spruce", &["solstice-cwa-alp-smrk", "solstice-cwa-alp-smrk-ridky", "solstice-cwa-alp-smrk-ridky-v1", "solstice-cwa-alp-smrk-v1", "solstice-cwa-smrk", "solstice-cwa-smrk-dvoj", "solstice-cwa-smrk-dvoj-v1", "solstice-cwa-smrk-medium", "solstice-cwa-smrk-medium-v1", "solstice-cwa-smrk-ridky", "solstice-cwa-smrk-ridky-v1", "solstice-cwa-smrk-v1", "solstice-cwa-smrk-vysoky", "solstice-cwa-smrk-vysoky-v1"]),
    ("Pear", &["solstice-cwa-pl-hrusen", "solstice-cwa-pl-hrusen2", "solstice-cwa-pl-hrusen2-v1", "solstice-cwa-pl-hrusen-v1"]),
    ("Apple", &["solstice-cwa-pl-jablon", "solstice-cwa-pl-jablon-v1", "solstice-cwa-jablon", "solstice-cwa-jablon-v1"]),
    ("Birch", &["solstice-cwa-briza", "solstice-cwa-briza-kriva", "solstice-cwa-briza-kriva-v1", "solstice-cwa-briza-rovna", "solstice-cwa-briza-rovna-v1", "solstice-cwa-briza-v1", "solstice-cwa-briza-v2"]),
    ("Beech", &["solstice-cwa-buk", "solstice-cwa-buk-v1", "solstice-cwa-buk-v2"]),
    ("Oak", &["solstice-cwa-dub", "solstice-cwa-dub-jiny", "solstice-cwa-dub-jiny-v1", "solstice-cwa-dub-jiny-v2", "solstice-cwa-dub-v1", "solstice-cwa-dub-v2"]),
    ("Fig", &["solstice-cwa-fikovnik", "solstice-cwa-fikovnik2", "solstice-cwa-fikovnik2-v1", "solstice-cwa-fikovnik-ker", "solstice-cwa-fikovnik-ker-v1", "solstice-cwa-fikovnik-v1"]),
    ("Hornbeam", &["solstice-cwa-habr", "solstice-cwa-habr-v1", "solstice-cwa-habr-v2"]),
    ("Juniper", &["solstice-cwa-jalovec", "solstice-cwa-jalovec-v1"]),
    ("Maple", &["solstice-cwa-javor", "solstice-cwa-javor-v1", "solstice-cwa-javor-v2"]),
    ("Rowan", &["solstice-cwa-jerabina", "solstice-cwa-jerabina-v1"]),
    ("Chestnut", &["solstice-cwa-kastan", "solstice-cwa-kastan-v1", "solstice-cwa-kastan-v2"]),
    ("Krovisko", &["solstice-cwa-krovisko-vysoke", "solstice-cwa-krovisko-vysoke-v1"]),
    ("Linden", &["solstice-cwa-lipa", "solstice-cwa-lipa-v1", "solstice-cwa-lipa-v2"]),
    ("Hazel", &["solstice-cwa-liskac", "solstice-cwa-liskac-v1"]),
    ("Aspen", &["solstice-cwa-osika", "solstice-cwa-osika-v1", "solstice-cwa-osika-v2"]),
    ("Stone pine", &["solstice-cwa-pinie", "solstice-cwa-pinie-v1"]),
    ("Dwarf spruce", &["solstice-cwa-smrcicicek", "solstice-cwa-smrcicicek-les", "solstice-cwa-smrcicicek-les-v1", "solstice-cwa-smrcicicek-v1"]),
    ("Poplar", &["solstice-cwa-topol", "solstice-cwa-topol2", "solstice-cwa-topol2-v1", "solstice-cwa-topol-v1"]),
    ("Blackthorn", &["solstice-cwa-trnka", "solstice-cwa-trnka-v1"]),
    ("Willow", &["solstice-cwa-vrba", "solstice-cwa-vrba-v1"]),
];
