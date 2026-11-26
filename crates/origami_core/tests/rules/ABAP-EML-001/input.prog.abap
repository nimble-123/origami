read entities of /DMO/I_Travel_M in local mode entity travel fields ( customer_id ) with corresponding #( keys ) result data(travels) failed data(failed).
modify entities of /dmo/i_travel_m
 entity travel
 create from create_travel
 reported reported
 failed failed
 mapped mapped.
READ ENTITIES OF /dmo/i_travel_m IN LOCAL MODE ENTITY travel BY \_booking ALL FIELDS WITH CORRESPONDING #( keys ) RESULT DATA(bookings).
MODIFY ENTITIES OF /dmo/i_travel_m IN LOCAL MODE ENTITY travel EXECUTE accept FROM CORRESPONDING #( keys ) RESULT DATA(accepted).
