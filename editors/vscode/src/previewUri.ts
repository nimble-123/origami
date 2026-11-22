// URIs of the read-only documents in the "Preview Changes" diff view:
// `origami-preview:/<name>?side=before|after&id=<n>`. No `vscode` import, so
// it can be unit-tested with node:test.
//
// The name is only a label (and its extension lets VS Code pick a language),
// so characters outside [A-Za-z0-9._-] become `_`; the URI then needs no
// escaping in its path.

export const SCHEME = "origami-preview";

export type Side = "before" | "after";

export interface PreviewRef {
  id: number;
  side: Side;
  name: string;
}

export function encode(ref: PreviewRef): string {
  const name = ref.name.replace(/[^A-Za-z0-9._-]/g, "_") || "document";
  return `${SCHEME}:/${name}?side=${ref.side}&id=${ref.id}`;
}

/** Undefined for anything that is not a preview URI. Accepts the query
 * percent-encoded, as `vscode.Uri.toString()` may write it. */
export function decode(uri: string): PreviewRef | undefined {
  const prefix = `${SCHEME}:/`;
  if (!uri.startsWith(prefix)) {
    return undefined;
  }
  const rest = uri.slice(prefix.length);
  const q = rest.indexOf("?");
  if (q < 0) {
    return undefined;
  }
  let query: URLSearchParams;
  try {
    query = new URLSearchParams(decodeURIComponent(rest.slice(q + 1)));
  } catch {
    return undefined;
  }
  const side = query.get("side");
  const id = Number(query.get("id"));
  if ((side !== "before" && side !== "after") || !Number.isSafeInteger(id) || id < 0) {
    return undefined;
  }
  return { id, side, name: rest.slice(0, q) };
}
