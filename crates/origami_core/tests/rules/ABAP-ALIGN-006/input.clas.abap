CLASS lcl_demo DEFINITION INHERITING FROM lcl_base.
  PUBLIC SECTION.
    " comment on first group of redefinitions
    METHODS if_any_interface~medium_method_name
      REDEFINITION.
    METHODS if_any_interface~short_name
      REDEFINITION.
    METHODS if_any_interface~extra_long_method_name
      REDEFINITION.

    " comment on second group of redefinitions
    METHODS if_other_interface~any_method_name
      FINAL REDEFINITION.
    METHODS if_other_interface~other_method_name
      FINAL REDEFINITION.

    METHODS
      if_other_interface~create REDEFINITION.

    " chains are aligned independently:
    METHODS:
      if_yet_another_interface~any_method_name
        REDEFINITION,
      if_yet_another_interface~other_method_name
        REDEFINITION,
      if_yet_another_interface~create
        REDEFINITION.

    METHODS: start REDEFINITION, finish FINAL REDEFINITION.
    METHODS: helper, run REDEFINITION, stop REDEFINITION.
ENDCLASS.
