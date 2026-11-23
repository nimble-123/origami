CLASS lcl_demo DEFINITION.
  PUBLIC SECTION.
    INTERFACES: if_any_interface, if_other_interface, if_yet_another_interface.

    " comment on first group of aliases
    ALIASES medium_method_name
      FOR if_any_interface~medium_method_name.
    ALIASES short_name
      FOR if_any_interface~short_name.
    ALIASES extra_long_method_name
      FOR if_any_interface~extra_long_method_name.

    " comment on second group of aliases
    ALIASES any_method_name
      FOR if_other_interface~any_method_name.
    ALIASES other_method_name
      FOR if_other_interface~other_method_name.

    ALIASES create
      FOR if_other_interface~create.

    " chains are aligned independently:
    ALIASES:
      first_method_name
        FOR if_yet_another_interface~first_method_name,
      second_method_name
        FOR if_yet_another_interface~second_method_name,
      last_method_name
        FOR if_yet_another_interface~last_method_name.
    ALIASES: run FOR if_any_interface~run, stop FOR if_any_interface~stop.
ENDCLASS.
