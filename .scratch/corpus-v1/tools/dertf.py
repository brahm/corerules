#!/usr/bin/env python3
"""Crude RTF -> text. Keeps \\par as newline and \\tab as TAB so table
structure (which these files express with tabs, not \\trowd) survives."""
import re, sys

def dertf(data: str) -> str:
    out, i, n = [], 0, len(data)
    skip_depth = None   # group depth at which a \* destination started
    depth = 0
    while i < n:
        c = data[i]
        if c == '{':
            depth += 1; i += 1
        elif c == '}':
            if skip_depth is not None and depth <= skip_depth:
                skip_depth = None
            depth -= 1; i += 1
        elif c == '\\':
            m = re.match(r"\\([a-zA-Z]+)(-?\d+)? ?", data[i:])
            if m:
                word, arg = m.group(1), m.group(2)
                i += m.end()
                if skip_depth is not None:
                    continue
                if word == 'par':
                    out.append('\n')
                elif word == 'tab':
                    out.append('\t')
                elif word in ('line', 'sect', 'page'):
                    out.append('\n')
                elif word in ('emdash', 'endash'):
                    out.append('-')
                elif word in ('lquote', 'rquote'):
                    out.append("'")
                elif word in ('ldblquote', 'rdblquote'):
                    out.append('"')
                elif word == 'u' and arg:
                    try:
                        out.append(chr(int(arg) % 65536))
                    except ValueError:
                        pass
                    if i < n and data[i] == '?':
                        i += 1
                elif word in ('fonttbl', 'colortbl', 'stylesheet', 'info',
                              'pict', 'object', 'header', 'footer', 'footnote'):
                    skip_depth = depth
            elif data[i:i+2] == "\\'":
                try:
                    out.append(bytes([int(data[i+2:i+4], 16)]).decode('cp1252', 'replace'))
                except ValueError:
                    pass
                i += 4
            elif data[i:i+2] == '\\*':
                skip_depth = depth
                i += 2
            elif i + 1 < n and data[i+1] in '\\{}':
                if skip_depth is None:
                    out.append(data[i+1])
                i += 2
            else:
                i += 1
        else:
            # raw CR/LF carry no meaning in RTF and must be dropped, not kept
            if skip_depth is None and c not in '\r\n':
                out.append(c)
            i += 1
    return ''.join(out)

if __name__ == '__main__':
    raw = open(sys.argv[1], 'r', encoding='cp1252', errors='replace').read()
    sys.stdout.write(dertf(raw))
