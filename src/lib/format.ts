export function bytes(b: number, digits = 1): string {
  if (!b) return "0 B";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = b;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  if (i === 0) return `${b} B`;
  return `${v >= 100 ? v.toFixed(0) : v.toFixed(digits)} ${u[i]}`;
}

/** Splits "12.3 GB" into number + unit for typographic emphasis. */
export function bytesParts(b: number): [string, string] {
  const [n, u] = bytes(b).split(" ");
  return [n, u ?? ""];
}

const now = () => Date.now() / 1000;

export function ago(ts: number): string {
  if (!ts) return "never";
  const s = Math.max(0, now() - ts);
  if (s < 45) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  const d = Math.round(s / 86400);
  if (d === 1) return "yesterday";
  if (d < 14) return `${d} days ago`;
  if (d < 60) return `${Math.round(d / 7)} weeks ago`;
  if (d < 365) return `${Math.round(d / 30)} months ago`;
  const y = Math.floor(d / 365);
  return y === 1 ? "over a year ago" : `${y} years ago`;
}

export function age(ts: number): string {
  if (!ts) return "";
  const d = Math.floor((now() - ts) / 86400);
  if (d < 1) return "today";
  if (d === 1) return "1 day";
  if (d < 14) return `${d} days`;
  if (d < 60) return `${Math.round(d / 7)} weeks`;
  if (d < 365) return `${Math.round(d / 30)} months`;
  const y = Math.floor(d / 365);
  return y === 1 ? "over a year" : `${y} years`;
}

export function until(ts: number): string {
  const s = ts - now();
  if (s <= 60) return "any moment";
  if (s < 3600) return `in ${Math.round(s / 60)} min`;
  if (s < 86400) return `in ${Math.round(s / 3600)} h`;
  return `in ${Math.round(s / 86400)} days`;
}

export function clock(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function dayLabel(ts: number): string {
  const d = new Date(ts * 1000);
  const today = new Date();
  const y = new Date();
  y.setDate(today.getDate() - 1);
  if (d.toDateString() === today.toDateString()) return "Today";
  if (d.toDateString() === y.toDateString()) return "Yesterday";
  return d.toLocaleDateString([], { weekday: "long", month: "long", day: "numeric" });
}

export function host(url: string | null): string {
  if (!url) return "";
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
}

/** C:\Users\me\Downloads\x -> ~\Downloads\x */
export function shortPath(p: string, home = ""): string {
  if (home && p.toLowerCase().startsWith(home.toLowerCase())) return "~" + p.slice(home.length);
  return p;
}

export function parent(p: string): string {
  const i = p.lastIndexOf("\\");
  return i > 0 ? p.slice(0, i) : p;
}

export function greeting(): string {
  const h = new Date().getHours();
  if (h < 5) return "Up late?";
  if (h < 12) return "Good morning.";
  if (h < 18) return "Good afternoon.";
  return "Good evening.";
}

export function plural(n: number, one: string, many = one + "s"): string {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}
