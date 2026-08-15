# Licencia y decisión de distribución

Registro de la decisión de licencia de este proyecto, el razonamiento detrás de ella y la ruta
prevista.

## Licencia actual

**Apache License 2.0.** El texto completo está en [`LICENSE`](../LICENSE).

Es la misma licencia que las otras ediciones de RootCause: mantener una sola licencia en toda la
familia evita que integrar código entre ediciones se convierta en un problema legal.

## Por qué Apache 2.0 y no MIT

Ambas son permisivas, pero Apache 2.0 añade dos cosas que MIT no tiene:

| Aspecto | MIT | Apache 2.0 |
|---|---|---|
| Concesión expresa de patentes | No | **Sí** |
| Cláusula de represalia por litigio de patentes | No | **Sí** |
| Requisito de declarar cambios | No | Sí |
| Protección de marcas | No | Sí (no concede derechos sobre el nombre) |

Para una herramienta de seguridad, la concesión de patentes importa: quien la adopte en un entorno
corporativo necesita saber que usarla no le expone a una reclamación posterior del autor.

## Por qué no una licencia copyleft

GPL o AGPL obligarían a liberar el código de cualquier obra derivada. Es una postura defendible,
pero cerraría la puerta a que un equipo interno integre partes de RootCause en su propio
instrumental sin publicar todo su código. El objetivo del proyecto es que **la técnica se use**, no
controlar quién la usa.

## Qué implica para quien lo utiliza

Puedes:

- usarlo en tu empresa, sin coste ni aviso previo,
- modificarlo y distribuir tu versión,
- integrarlo en software propietario.

Debes:

- conservar el aviso de copyright y la licencia,
- declarar los cambios significativos que hagas,
- no usar el nombre «RootCause» para promocionar tu derivado sin permiso.

## Decisión de distribución binaria

**No se distribuyen binarios firmados ni notarizados por Apple.** La vía recomendada es compilar
desde el código.

| Razón | Detalle |
|---|---|
| Auditabilidad | Un binario opaco de una herramienta de seguridad es una contradicción |
| Coherencia | El producto insiste en verificar la firma de otros binarios; distribuir uno sin firma obliga a decirlo en voz alta en vez de esconderlo |
| Coste | La notarización exige cuenta de Apple Developer de pago |

Consecuencia práctica y honesta: **Gatekeeper avisará** al abrir el `.app` descargado, y hay que
autorizarlo a mano. Está documentado en el `LÉEME.txt` de la propia imagen de disco.

## Ruta prevista

Si el proyecto llegara a distribuirse formalmente, la secuencia sería:

1. Cuenta de Apple Developer y firma con Developer ID.
2. Notarización y *stapling* del `.dmg`.
3. Cask de Homebrew en un tap propio, con SHA-256 verificable.

Nada de eso cambia la licencia del código.

## Dependencias y sus licencias

Las dependencias se mantienen deliberadamente pocas. Todas son permisivas (MIT, Apache-2.0 o
equivalentes) y compatibles con Apache 2.0. La lista completa está en `Cargo.toml` y `Cargo.lock`;
para revisarla:

```bash
cargo tree --depth 1
```
