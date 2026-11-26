// The code the website shows, in one place. The sample class itself is
// site/samples/zcl_flight_check.clas.abap (with its origami.toml); these are
// the pieces of it that the page shows on their own. Formatted output is
// never written here: the page and site/fill.mjs ask origami for it.

/** line_width in site/samples/origami.toml. */
export const SAMPLE_WIDTH = 72;

/** The call the check follows token by token. */
export const CHECK_SOURCE =
  "DATA(ls_rule) = zcl_flight_rules=>get( iv_carrier = is_flight-carrid iv_date = sy-datum iv_mode = 'A' ).";

/** Statements as someone typed them: lower-case keywords, two spaces, one
 * long condition. */
export const LAYOUT_SOURCE = [
  "data lv_count  type i.",
  "if ls_rule-active = abap_true and lv_count = 0 and is_flight-seatsmax > 0.",
  "rv_ok = abap_true.",
  "endif.",
].join("\n");
