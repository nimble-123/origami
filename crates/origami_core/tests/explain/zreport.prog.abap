REPORT zreport.
START-OF-SELECTION. PERFORM main USING lv_first_parameter lv_second_parameter CHANGING lv_third_parameter lv_fourth_parameter lv_fifth_parameter.
FORM main USING iv_first TYPE i iv_second TYPE string CHANGING cv_third TYPE i cv_fourth TYPE string cv_fifth TYPE i cv_sixth TYPE i.
  WRITE: / iv_first, iv_second. " output
ENDFORM.
