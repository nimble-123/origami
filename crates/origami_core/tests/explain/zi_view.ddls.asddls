@AbapCatalog.viewEnhancementCategory: [#NONE] @AccessControl.authorizationCheck: #CHECK
define view entity ZI_View as select from sflight association [0..1] to ZI_Carrier as _Carrier on $projection.carrid = _Carrier.carrid {
key carrid, key connid, fldate,
case when price > 1000 then 'expensive' when price > 500 then 'medium' else 'cheap' end as price_class,
_Carrier
}
