report zdemo message-id zz.
parameters p_carr type s_carr_id obligatory.
initialization.
p_carr = 'LH'.
at selection-screen.
if p_carr is initial.
message e001.
endif.
start-of-selection.
perform load_data.
write: / 'Done'.
get time.
get pernr late.
write pernr-pernr.
form load_data.
data lt_flights type standard table of sflight with empty key.
endform.
