"""Convert single-line fonts (Hershey .jhf + .hmp, EMS SVG fonts) to the
practice-sheet stroke format: printable ASCII only, y up from the baseline.

Output format (one file per font):
  # comments
  metrics <x_height> <ascender> <descender> <slant_degrees>
  <code> <advance> <x,y x,y ...>|<x,y ...>     (one line per character)
"""
import math, os, re, sys, html

HERSHEY = sys.argv[1]   # vector-text-hershey crate dir
SVGDIR = sys.argv[2]    # inkscape svg_fonts dir
OUT = sys.argv[3]

def jhf_glyphs():
    lines = open(os.path.join(HERSHEY, 'data/hershey.jhf'), encoding='latin-1').read().split('\n')
    glyphs = {}; cur = None
    for l in lines:
        if not l: continue
        if l[:5].strip().isdigit() and (cur is None or len(cur[1]) >= cur[2]):
            cur = [int(l[:5]), l[8:], int(l[5:8]) * 2]; glyphs[cur[0]] = cur
        else:
            cur[1] += l
    return glyphs

def hmp(name):
    nums = []
    for l in open(os.path.join(HERSHEY, 'data/mappings', name + '.hmp')):
        p = l.split()
        if p:
            a, b = int(p[0]), int(p[1])
            nums += [a] if b == 0 else list(range(a, b + 1))
    return nums[:95]

def from_jhf(name, glyphs):
    out = {}
    for code, num in zip(range(32, 127), hmp(name)):
        g = glyphs.get(num)
        data = g[1] if g else 'JZJ]Z]'   # underscore missing: a line
        left, right = ord(data[0]) - 82, ord(data[1]) - 82
        strokes = [[]]
        for i in range(2, len(data), 2):
            pair = data[i:i + 2]
            if pair == ' R':
                strokes.append([])
            else:
                x, y = ord(pair[0]) - 82, ord(pair[1]) - 82
                strokes[-1].append((x - left, 9 - y))
        out[code] = (right - left, [s for s in strokes if s])
    return out

def cubic(p0, p1, p2, p3, n):
    return [tuple((1-t)**3*a + 3*(1-t)**2*t*b + 3*(1-t)*t*t*c + t**3*d for a, b, c, d in zip(p0, p1, p2, p3))
            for t in (i / n for i in range(1, n + 1))]

def parse_d(d):
    toks = re.findall(r'[MLCZmlcz]|-?[\d.]+(?:e-?\d+)?', d)
    strokes = []; i = 0; cmd = None; cur = (0, 0); start = (0, 0)
    num = lambda: float(toks[i])
    while i < len(toks):
        t = toks[i]
        if t.isalpha():
            cmd = t; i += 1
            if cmd in 'Zz':
                strokes[-1].append(start); continue
        if cmd == 'M':
            cur = (float(toks[i]), float(toks[i+1])); i += 2
            strokes.append([cur]); start = cur; cmd = 'L'
        elif cmd == 'L':
            cur = (float(toks[i]), float(toks[i+1])); i += 2
            strokes[-1].append(cur)
        elif cmd == 'C':
            c = [float(x) for x in toks[i:i+6]]; i += 6
            p3 = (c[4], c[5])
            n = max(2, int(math.dist(cur, p3) / 15) + 2)
            strokes[-1] += cubic(cur, (c[0], c[1]), (c[2], c[3]), p3, n); cur = p3
        else:
            raise ValueError('unsupported ' + cmd)
    return strokes

def from_svg(path):
    text = open(path, encoding='utf-8').read()
    default_adv = float(re.search(r'<font [^>]*horiz-adv-x="([\d.]+)"', text).group(1))
    out = {}
    for m in re.finditer(r'<glyph\s([^>]*)/>', text, re.S):
        attrs = dict(re.findall(r'([\w-]+)="([^"]*)"', m.group(1)))
        u = html.unescape(attrs.get('unicode', ''))
        if len(u) != 1 or not 32 <= ord(u) <= 126: continue
        adv = float(attrs.get('horiz-adv-x', default_adv))
        strokes = parse_d(attrs['d']) if 'd' in attrs else []
        out[ord(u)] = (adv, [s for s in strokes if len(s) >= 1])
    return out

def metrics(glyphs):
    import statistics
    def extent(chars, top):
        vals = []
        for c in chars:
            pts = [p[1] for s in glyphs.get(ord(c), (0, []))[1] for p in s]
            if pts:
                vals.append(max(pts) if top else -min(pts))
        return statistics.median(vals)
    # Medians, so one letter's flourish doesn't stretch the guides.
    x_height = extent('acemnorsuvwxz', True)
    ascender = extent('bdhkl', True) - x_height
    descender = extent('gjpqy', False)
    # Slant: length-weighted mean angle of near-vertical segments of letters.
    num = den = 0.0
    for c in 'bdhiklmnprtu':
        for s in glyphs.get(ord(c), (0, []))[1]:
            for (x0, y0), (x1, y1) in zip(s, s[1:]):
                dx, dy = x1 - x0, y1 - y0
                if abs(dy) > 2.5 * abs(dx) and abs(dy) > 0:
                    if dy < 0: dx, dy = -dx, -dy
                    length = math.hypot(dx, dy)
                    num += math.degrees(math.atan2(dx, dy)) * length; den += length
    slant = num / den if den else 0.0
    return x_height, ascender, descender, slant

def write(name, glyphs, source):
    m = metrics(glyphs)
    with open(os.path.join(OUT, name + '.strokes'), 'w') as f:
        f.write('# ' + source + '\n')
        f.write('metrics %.1f %.1f %.1f %.1f\n' % m)
        for code in range(32, 127):
            adv, strokes = glyphs.get(code, (m[0], []))
            fmt = lambda v: ('%.1f' % v).rstrip('0').rstrip('.')
            f.write('%d %s %s\n' % (code, fmt(adv), '|'.join(' '.join(fmt(x) + ',' + fmt(y) for x, y in s) for s in strokes)))
    print('%-22s x=%.0f asc=%.2f desc=%.2f slant=%.1f' % (name, m[0], m[1]/m[0], m[2]/m[0], m[3]))

g = jhf_glyphs()
for name, src in [('romans', 'Hershey Roman Simplex'), ('scripts', 'Hershey Script Simplex'),
                  ('romanc', 'Hershey Roman Complex'), ('scriptc', 'Hershey Script Complex'),
                  ('italicc', 'Hershey Italic Complex'), ('gotheng', 'Hershey Gothic English'),
                  ('gothger', 'Hershey Gothic German'), ('gothita', 'Hershey Gothic Italian')]:
    write(name, from_jhf(name, g), src)
for name in ['EMSAllure', 'EMSElfin', 'EMSFelix', 'EMSNixish', 'EMSNixishItalic', 'EMSReadability', 'EMSReadabilityItalic', 'EMSTech']:
    write(name, from_svg(os.path.join(SVGDIR, name + '.svg')), name + ' (SIL Open Font License 1.1)')
