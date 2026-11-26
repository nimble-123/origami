IF    lv_first_condition_value = abap_true
   OR ( lv_second_value = 1 AND lv_third_value IS INITIAL )
   OR lv_fourth_value BETWEEN 1 AND 10.
ENDIF.
IF     lv_a   = 1
   AND lv_bb  = 2
   AND lv_c  <> 3.
ENDIF.
IF a = 1 AND b = 2.
ENDIF.
IF    lv_a   = 1
   OR lv_bb  = 2
   OR lv_c  IS NOT INITIAL.
ELSEIF     lv_a  = 2
       AND lv_bb = 3
       AND lv_c  = 4.
ENDIF.
IF        lv_a  = 1
      AND lv_bb = 2
   OR lv_c <> 3.
ENDIF.
IF     NOT lv_a           = 1
   AND     lv_bb    BETWEEN 1 AND 10
   AND     lv_c NOT      IN lt_range
   AND     line_exists( lt_tab[ a = 1 ] ).
ENDIF.
IF     a = 1
   AND b_with_a_very_long_name_that_is_longer_than_twenty = 2
   AND c = 3.
ENDIF.
IF lv_a = 1
  AND ( lv_value_number_one = lv_comparison_value_one OR lv_value_number_two = lv_comparison_value_two OR lv_x = 3 ).
ENDIF.
IF     lv_a = 1
   AND
   " the second one
   lv_bb = 2
   AND lv_c = 3. " done
ENDIF.
IF     lv_a = 1
   AND lo_some_object->get_a_rather_long_attribute_name( iv_parameter = lv_value_with_long_name ) = lv_another_variable.
ENDIF.
WHILE     lv_a   = 1
      AND lv_bb  = 2
      AND lv_c  <> 3.
ENDWHILE.
CHECK     lv_a   = 1
      AND lv_bb  = 2
      AND lv_c  <> 3.
ASSERT     lv_a   = 1
       AND lv_bb  = 2
       AND lv_c  <> 3.
LOOP AT lt_all INTO DATA(ls_other)
     WHERE     user_name   = ls_self-user_name
           AND plant       = ls_self-plant
           AND valid_from <> ls_self-valid_from.
ENDLOOP.
LOOP AT lt_all INTO DATA(ls_other) WHERE user_name = ls_self-user_name AND plant = ls_self-plant.
ENDLOOP.
LOOP AT lt_all_entries_of_the_table INTO DATA(ls_other_entry)
     WHERE user_name = ls_self-user_name AND plant = ls_self-plant.
ENDLOOP.
DELETE lt_all WHERE     user_name   = ls_self-user_name
                    AND plant       = ls_self-plant
                    AND valid_from <> ls_self-valid_from.
lv_x = COND #( WHEN a  = 1
                AND bb = 2
                AND c  = 3
               THEN 1
               ELSE 2 ).
lv_y = COND #( WHEN a  = 1
                 OR bb = 2
                 OR c  = 3 THEN
                 1
               WHEN d = 4 THEN
                 2 ).
SELECT *
  FROM sflight
  WHERE carrid = @lv_carrier AND connid = @lv_connid AND fldate = @lv_date
  INTO TABLE @DATA(lt_rows).
IF     lv_flag = abap_true
   AND (    lv_value_number_one = lv_comparison_value_one
         OR lv_value_number_two = lv_comparison_value_two
         OR lv_value_three      = lv_comparison_value_three ).
ENDIF.
IF     lv_flag       = abap_true
   AND lv_other_flag = abap_false
   AND NOT ( lv_value_number_one = lv_comparison_value_one AND lv_value_number_two = lv_comparison_value_two_long ).
ENDIF.
