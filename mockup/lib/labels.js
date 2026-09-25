/*
 * A small label memory for entities the console has already seen.
 *
 * Why this exists: the reads are cursor-paginated with no per-id route, so a
 * selected engagement or resource that is not on the current page has NO name
 * available from the wire. The pages used to print "Selected engagement outside
 * this page" — a placeholder where a name belongs, shown most often right after
 * "Open saved resource", when the reader has just been looking at that exact
 * row. The row WAS named a moment ago; forgetting it is what made the
 * placeholder, not the wire shape.
 *
 * So each read records the label of every row it returns, and the pickers read
 * it back. It is a cache, not authority: an id with no recorded label still
 * renders the honest placeholder, and nothing here decides whether an id is
 * valid — the server does that when the read is made.
 */
const seen = new Map();

/** Record the display label for an id, if we do not already know one. */
export function remember(id, label) {
  if (id && label && !seen.has(id)) seen.set(id, label);
}

/** The last label recorded for an id, or undefined when never seen. */
export function labelFor(id) {
  return id ? seen.get(id) : undefined;
}
