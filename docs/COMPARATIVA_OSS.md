# Comparativa con el open source de seguridad para macOS

Dónde encaja RootCause frente a las herramientas que ya existen, qué conviene tomar de cada una y
qué no.

## El mapa

| Herramienta | Qué hace | Licencia | Solapamiento con RootCause |
|---|---|---|---|
| **KnockKnock** (Objective-See) | Lista lo que se ejecuta persistentemente en macOS | Objective-See | **Alto** — misma superficie |
| **BlockBlock** (Objective-See) | Alerta en tiempo real cuando algo instala persistencia | Objective-See | Medio — misma superficie, otro modelo |
| **LuLu** (Objective-See) | Firewall de salida por aplicación | GPL-3.0 | Bajo — RootCause observa, no bloquea |
| **osquery** | Consulta el sistema como si fuera SQL | Apache-2.0 | Medio — mismos datos, sin veredicto |
| **Velociraptor** | Recolección forense y *hunting* a escala | AGPL-3.0 | Medio — orientado a flota, no a un equipo |
| **Santa** (North Pole Security) | Control de ejecución de binarios | Apache-2.0 | Bajo — decide qué puede ejecutarse |
| **Wazuh** | SIEM/HIDS con agente | GPL-2.0 | Medio — requiere servidor |
| **ClamAV** | Antivirus por firmas | GPL-2.0 | Nulo — RootCause no usa firmas |

## Qué aporta RootCause que no aportan las demás

1. **Una foto con memoria.** KnockKnock te dice qué persiste *ahora*. RootCause te dice **qué
   cambió** respecto a un estado que tú aceptaste como bueno, y lo sigue diciendo hasta que lo
   reconoces. Esa diferencia es la mitad del producto.
2. **Siete superficies en una vista.** Persistencia, controles nativos, XProtect, TCC, procesos,
   red y almacenamiento correlacionados en un solo veredicto, en vez de siete herramientas.
3. **Evidencia del comando.** Cada control muestra la salida cruda de `spctl`, `csrutil` o
   `fdesetup`. No hay que confiar en la interpretación de la herramienta.
4. **Sin servidor ni agente permanente.** Se abre, se mira, se cierra. Ni flota, ni consola central,
   ni cuenta.

## Qué NO intenta ser

- **No es Santa.** No decide qué puede ejecutarse.
- **No es LuLu.** No bloquea conexiones; muestra el comando `pfctl` y te deja decidir.
- **No es osquery.** No expone un lenguaje de consulta; da un veredicto con explicación.
- **No es Velociraptor.** No está pensado para una flota, sino para un equipo concreto.

## Qué conviene tomar de cada una

Con el semáforo de licencias por delante, porque tomar código y tomar ideas son cosas distintas:

| Fuente | Qué tomar | ¿Código o idea? |
|---|---|---|
| osquery (Apache-2.0) | Su catálogo de rutas de persistencia, que es exhaustivo | **Código compatible** |
| Santa (Apache-2.0) | Su modelo de reglas y cómo explican una decisión | **Código compatible** |
| KnockKnock | La taxonomía de superficies de persistencia en macOS | **Solo la idea** (licencia propia) |
| LuLu (GPL-3.0) | Cómo presentan una conexión sospechosa a un usuario no técnico | **Solo la idea** — GPL es incompatible con Apache 2.0 en un producto permisivo |
| Wazuh (GPL-2.0) | Sus reglas de correlación | **Solo la idea** |

Detalle del semáforo en [`GUIA_COMPETENCIA.md`](GUIA_COMPETENCIA.md).

## Oportunidades priorizadas

Lo que más valor añadiría, en orden:

1. **Ampliar el catálogo de superficies de persistencia** siguiendo el de osquery: extensiones de
   sistema, perfiles de configuración, `emond` heredado, `at`.
2. **Verificación de integridad de aplicaciones** en `/Applications` contra su firma, al estilo de
   lo que hace Santa antes de dejar ejecutar.
3. **Exportación compatible** con formatos que otras herramientas ya leen, para que RootCause sea
   el primer paso de una investigación y no un callejón sin salida.

## Posicionamiento honesto

RootCause no compite con un EDR ni pretende sustituir a las herramientas de Objective-See, que
llevan años y tienen acceso a APIs privilegiadas que este producto no usa. Su hueco es el de
**la primera pregunta**: *¿qué cambió en este Mac desde la última vez que lo miré?*
