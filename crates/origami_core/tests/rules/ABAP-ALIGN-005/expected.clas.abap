CLASS ltc_demo DEFINITION FINAL FOR TESTING RISK LEVEL HARMLESS DURATION SHORT.
  PRIVATE SECTION.
    " comment on first group of test methods
    METHODS test_setup             FOR TESTING.
    METHODS test_functionality     FOR TESTING.
    METHODS test_display           FOR TESTING.

    " comment on second group of test methods
    METHODS any_test_method_name   FOR TESTING RAISING cx_static_check.
    METHODS other_test_method_name FOR TESTING RAISING cx_static_check.

    METHODS test_invalid_input     FOR TESTING.

    METHODS setup.
    METHODS test_a   FOR TESTING.
    METHODS test_bbb FOR TESTING.

    " chains are aligned independently:
    METHODS: first_test_method  FOR TESTING,
             second_test_method FOR TESTING RAISING cx_message,
             last_test_method   FOR TESTING.

    METHODS: teardown,
             test_c    FOR TESTING,
             test_dddd FOR TESTING.
    METHODS: helper, test_e FOR TESTING.
ENDCLASS.
