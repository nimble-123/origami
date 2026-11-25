@AbapCatalog.viewEnhancementCategory: [#NONE]

define view entity ZI_Flights
  as select from spfli

{
  key carrid,
  key connid,

      cityfrom
}

where carrid = 'LH'
group by carrid, connid, cityfrom
having count(*) > 1

union all select from sflight { key carrid, key connid, cityfrom }
where carrid = 'AA'
// sold out
  and connid > '0100'
group by carrid,
         connid,
         cityfrom
