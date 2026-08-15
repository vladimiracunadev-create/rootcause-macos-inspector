# Rust para RootCause

Por qué el producto está en Rust y cómo se usa el lenguaje aquí. Pensado para quien va a leer el
código o a contribuir.

## Por qué Rust

| Necesidad del producto | Qué aporta Rust |
|---|---|
| Un sensor que corre en el equipo del usuario | Binario nativo sin runtime ni intérprete que instalar |
| Refrescos cada pocos segundos sin molestar | Sin recolector de basura: no hay pausas impredecibles |
| Manejar salidas de comandos del sistema | El compilador obliga a tratar el caso en que el comando falle |
| Una herramienta de seguridad | Sin desbordamientos de búfer ni *use-after-free* por construcción |
| Dos ediciones desde un código | *Feature flags* en tiempo de compilación, sin coste en ejecución |

La razón decisiva es la cuarta: en una herramienta que parsea salidas de comandos ajenos, la clase
entera de vulnerabilidades de memoria desaparece del mapa.

## Cómo se usa aquí

### `Result` en todo lo que toca el sistema

Ninguna función que hable con macOS devuelve un valor directo. El patrón es siempre el mismo:
quien llama decide qué hacer si falla, y el fallo de una superficie nunca tumba la captura.

```rust
let connections = macos::lsof_connections()
    .map(|raw| network::parse_lsof_field_output(&raw, &process_paths))
    .unwrap_or_default();
```

Si `lsof` no está o no tiene permiso, la sección queda vacía y se declara. El resto de la foto
sigue siendo útil.

### `let ... else` para salir temprano

Reduce el anidamiento en el código de parseo, que es donde más se acumula:

```rust
let Some(ip) = line.split_once('(').and_then(|(_, rest)| rest.split_once(')')) else {
    continue;
};
```

### Enums que no admiten estados imposibles

`Severity`, `RiskLevel`, `CodeSignature` y `PersistenceChange` son enums, no cadenas ni enteros. Un
proceso no puede tener severidad `"critico"` mal escrita, y el compilador obliga a cubrir cada
variante al clasificar.

`Severity` deriva `Ord`, así que ordenar por gravedad es `sort_by_key(|x| Reverse(x.severity))` en
vez de una tabla de prioridades a mano.

### Canales en vez de bloqueos compartidos

El motor y la interfaz no comparten estado con `Mutex`. Se mandan mensajes:

```rust
enum Command { Refresh, AcceptPersistence, CleanCaches { dry_run: bool }, /* … */ }
enum EngineEvent { Snapshot(Box<SystemSnapshot>), Failed(String), /* … */ }
```

No hay posibilidad de interbloqueo porque no hay nada que bloquear. El precio es copiar la captura
al pasarla; a cinco segundos de intervalo, es irrelevante.

### `serde` como columna vertebral

Los modelos derivan `Serialize`/`Deserialize`, y eso da cuatro cosas a la vez: la exportación JSON,
la persistencia de incidentes en SQLite, la configuración en disco y el `--json` de la CLI. Un solo
modelo, cuatro usos.

`#[serde(default)]` en los campos nuevos permite leer un JSON de una versión anterior sin romperlo.

## Dependencias y por qué cada una

| Crate | Para qué | Por qué esa |
|---|---|---|
| `eframe`/`egui` | Interfaz gráfica | Modo inmediato: la UI es una función del estado, sin sincronización de widgets |
| `rusqlite` (`bundled`) | Historial y baselines | SQLite compilado dentro: cero dependencias de sistema |
| `serde`/`serde_json` | Serialización | Estándar de facto |
| `chrono` | Fechas | Manejo correcto de zonas horarias y RFC 3339 |
| `sysinfo` | Métricas de procesos | Multiplataforma y mantenida |
| `plist` | Leer los `.plist` de launchd | Formato nativo de macOS, binario y XML |
| `anyhow` | Errores con contexto | Un mensaje que explica qué se intentaba hacer |
| `dirs` | Rutas del usuario | Respeta las convenciones de cada sistema |
| `walkdir` | Recorrer cachés | Recorrido con control de profundidad y enlaces |

No hay cliente HTTP: el adaptador de IA opcional usa `curl`, que viene con macOS. Añadir un stack
HTTP completo por una función opcional y apagada no compensa.

## Convenciones del repositorio

- **Comentarios que explican el porqué**, no el qué. Si el código dice *qué* hace, el comentario
  dice *por qué* así.
- **Tests que describen comportamiento**, con nombres en español que se leen como una frase:
  `un_pico_de_cpu_no_dispara_nada`, `control_desconocido_no_se_pinta_de_verde`.
- **Cada falso positivo corregido deja un test** que documenta el caso para que no vuelva.
- `cargo fmt` y `clippy -D warnings` no se negocian: la CI los exige.

## Para empezar a contribuir

```bash
./scripts/verify-environment.sh   # comprobar el entorno
cargo test --all-features         # los 112 tests
./scripts/ci-local.sh             # lo mismo que hará la CI
```

Por dónde entrar según lo que quieras tocar:

| Quiero… | Empieza por |
|---|---|
| Añadir una superficie | `src/services/` + registrar en `inspector.rs` |
| Ajustar una heurística | `src/services/anomaly.rs` y sus tests |
| Cambiar cómo se clasifica | `src/services/rules.rs` |
| Tocar la interfaz | `src/app.rs` (busca `draw_<sección>`) |
| Añadir un comando | `src/cli.rs` |
