READ ENTITIES OF /DMO/I_Travel_M IN LOCAL MODE
    ENTITY travel
      FIELDS ( customer_id ) WITH CORRESPONDING #( keys )
      RESULT DATA(travels)
    FAILED DATA(failed).
MODIFY ENTITIES OF /dmo/i_travel_m
    ENTITY travel
      CREATE FROM create_travel
    REPORTED reported
    FAILED failed
    MAPPED mapped.
READ ENTITIES OF /dmo/i_travel_m IN LOCAL MODE
    ENTITY travel
      BY \_booking ALL FIELDS WITH CORRESPONDING #( keys )
      RESULT DATA(bookings).
MODIFY ENTITIES OF /dmo/i_travel_m IN LOCAL MODE
    ENTITY travel
      EXECUTE accept FROM CORRESPONDING #( keys )
      RESULT DATA(accepted).
