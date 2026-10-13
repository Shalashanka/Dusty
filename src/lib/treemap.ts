/** Squarified treemap (Bruls, Huizing, van Wijk). */

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function squarify<T>(items: T[], value: (t: T) => number, box: Rect): (Rect & { item: T })[] {
  const total = items.reduce((a, t) => a + value(t), 0);
  if (!total || box.w <= 0 || box.h <= 0) return [];
  const scale = (box.w * box.h) / total;
  const nodes = items.map((item) => ({ item, area: value(item) * scale })).filter((n) => n.area > 0);
  const out: (Rect & { item: T })[] = [];
  let rect = { ...box };
  let row: typeof nodes = [];

  const worst = (r: typeof nodes, side: number) => {
    const s = r.reduce((a, n) => a + n.area, 0);
    let max = 0;
    let min = Infinity;
    for (const n of r) {
      max = Math.max(max, n.area);
      min = Math.min(min, n.area);
    }
    const s2 = s * s;
    const side2 = side * side;
    return Math.max((side2 * max) / s2, s2 / (side2 * min));
  };

  const layout = (r: typeof nodes) => {
    const s = r.reduce((a, n) => a + n.area, 0);
    if (rect.w >= rect.h) {
      const w = s / rect.h;
      let y = rect.y;
      for (const n of r) {
        const h = n.area / w;
        out.push({ x: rect.x, y, w, h, item: n.item });
        y += h;
      }
      rect = { x: rect.x + w, y: rect.y, w: rect.w - w, h: rect.h };
    } else {
      const h = s / rect.w;
      let x = rect.x;
      for (const n of r) {
        const w = n.area / h;
        out.push({ x, y: rect.y, w, h, item: n.item });
        x += w;
      }
      rect = { x: rect.x, y: rect.y + h, w: rect.w, h: rect.h - h };
    }
  };

  for (const n of nodes) {
    const side = Math.min(rect.w, rect.h);
    if (!row.length || worst([...row, n], side) <= worst(row, side)) {
      row.push(n);
    } else {
      layout(row);
      row = [n];
    }
  }
  if (row.length) layout(row);
  return out;
}
