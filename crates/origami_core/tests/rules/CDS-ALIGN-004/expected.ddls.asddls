define view ZI_Names
(
  Id,
  Name
)
  as select from t

{
  key id,

      name
}

group by id,
         name
