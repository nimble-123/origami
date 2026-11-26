REPORT zdemo.
"tax is added later
lv_tax = lv_total * lv_rate. "rounded
"lv_old = 1.
"! ABAP Doc stays
"-------------------
"* starred
"" double
lv_tax = 1. "#EC NEEDED
IF lv_tax > 0.
  IF lv_rate > 0.
   "a comment line that would pass column 255 when indented keeps its text as written and is indented only as far as it fits, so that the line stays within the 255 characters that ABAP allows; the statements around it are formatted as usual, like this one
    lv_tax = 1.
  ENDIF.
ENDIF.
