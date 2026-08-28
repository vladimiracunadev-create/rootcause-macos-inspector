#!/usr/bin/env python3
"""Genera la versión PDF de la documentación de sistema de RootCause.

Fuente única: los Markdown de ``docs/system-documentation/``.
Salida:       un PDF por documento en ``docs/system-documentation/pdf/``.

Los Markdown son la única fuente de verdad. Este script NO edita los .md: los
lee, los convierte a HTML con una hoja de estilo pensada para papel y los
renderiza a PDF. Si un documento cambia, basta con volver a ejecutar el script.

Uso
---
    python scripts/build-docs-pdf.py                 # todos los documentos
    python scripts/build-docs-pdf.py 07 11           # solo los que empiecen por 07 y 11
    python scripts/build-docs-pdf.py --check         # solo comprueba dependencias
    python scripts/build-docs-pdf.py --out OTRA/RUTA # directorio de salida alternativo

Requisitos
----------
    pip install markdown xhtml2pdf

Limitaciones conocidas (deliberadas y documentadas, no defectos silenciosos)
---------------------------------------------------------------------------
* Los diagramas Mermaid **no se renderizan como imagen**: el motor de PDF no
  ejecuta JavaScript. Se incluyen como bloque de código monoespaciado con un
  aviso, de modo que el PDF sigue siendo autocontenido y legible, y quien quiera
  el diagrama renderizado tiene la fuente para pegarla en cualquier visor
  Mermaid. La alternativa —depender de Node y del CLI de mermaid— haría el
  script no reproducible en un equipo limpio.
* Las tablas muy anchas se reducen de tamaño de fuente y parten palabras largas
  para no desbordar el ancho de página.
* Los enlaces relativos entre documentos (``03-architecture.md``) se conservan
  como texto; en el PDF no navegan a otro archivo.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import html
import io
import re
import sys
from pathlib import Path

# ── Metadatos del sistema documentado ────────────────────────────────────────
SYSTEM_NAME = "RootCause macOS Inspector"
DOCS_DIRNAME = Path("docs") / "system-documentation"
PDF_DIRNAME = "pdf"

# Un documento se considera "largo" —y por tanto merece índice propio— a partir
# de este número de encabezados de nivel 2.
TOC_MIN_H2 = 4


# ── Dependencias opcionales ──────────────────────────────────────────────────
def _load_dependencies():
    """Importa markdown y xhtml2pdf, con un mensaje útil si faltan."""
    missing = []
    try:
        import markdown  # noqa: F401
    except ImportError:
        missing.append("markdown")
    try:
        from xhtml2pdf import pisa  # noqa: F401
    except ImportError:
        missing.append("xhtml2pdf")

    if missing:
        print(
            "Faltan dependencias: " + ", ".join(missing) + "\n"
            "Instálalas con:\n\n    pip install " + " ".join(missing) + "\n",
            file=sys.stderr,
        )
        return None, None

    import markdown
    from xhtml2pdf import pisa

    return markdown, pisa


# ── Utilidades de repositorio ────────────────────────────────────────────────
def repo_root() -> Path:
    """Raíz del repositorio, deducida de la ubicación de este script."""
    return Path(__file__).resolve().parent.parent


def read_version(root: Path) -> str:
    """Versión declarada en Cargo.toml. Fuente única de verdad."""
    cargo = root / "Cargo.toml"
    try:
        for line in cargo.read_text(encoding="utf-8").splitlines():
            match = re.match(r'^version\s*=\s*"([^"]+)"', line.strip())
            if match:
                return match.group(1)
    except OSError:
        pass
    return "desconocida"


def read_commit(root: Path) -> str:
    """Commit corto analizado, leído de .git sin invocar git."""
    head = root / ".git" / "HEAD"
    try:
        content = head.read_text(encoding="utf-8").strip()
        if content.startswith("ref:"):
            ref = content.split(" ", 1)[1].strip()
            sha = (root / ".git" / ref).read_text(encoding="utf-8").strip()
        else:
            sha = content
        return sha[:7]
    except OSError:
        return "desconocido"


def document_title(markdown_text: str, fallback: str) -> str:
    """Primer encabezado H1 del documento; si no hay, el nombre del archivo."""
    for line in markdown_text.splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return fallback


# ── Preprocesado del Markdown ────────────────────────────────────────────────
MERMAID_BLOCK = re.compile(r"```mermaid\n(.*?)```", re.DOTALL)


def replace_mermaid_blocks(text: str) -> tuple[str, int]:
    """Sustituye los bloques ```mermaid por un aviso + el código fuente.

    Devuelve el texto transformado y cuántos bloques se sustituyeron, para poder
    informarlo al final en vez de dejar una degradación silenciosa.
    """
    count = 0

    def _replace(match: "re.Match[str]") -> str:
        nonlocal count
        count += 1
        source = match.group(1).rstrip()
        return (
            '<div class="diagram">\n'
            '<p class="diagram-label">Diagrama Mermaid '
            f"#{count} — código fuente (no renderizado en PDF)</p>\n"
            f"<pre class=\"diagram-code\">{html.escape(source)}</pre>\n"
            "</div>\n"
        )

    return MERMAID_BLOCK.sub(_replace, text), count


def build_toc(markdown_text: str) -> str:
    """Índice de los encabezados H2 del documento, o cadena vacía si es corto."""
    headings = [
        line[3:].strip()
        for line in markdown_text.splitlines()
        if line.startswith("## ")
    ]
    if len(headings) < TOC_MIN_H2:
        return ""
    # Los encabezados del documento ya llevan su propia numeración ("1. Motor y
    # ubicación"), así que el índice NO debe añadir otra: se emite como párrafos.
    items = "\n".join(f'<p class="toc-item">{html.escape(h)}</p>' for h in headings)
    return (
        '<div class="toc">\n<p class="toc-title">Contenido de este documento</p>\n'
        f"{items}\n</div>\n<pdf:nextpage />\n"
    )


# ── Plantilla y estilos ──────────────────────────────────────────────────────
# Estilos pensados para papel: cuerpo compacto, tablas que no desbordan, código
# legible en monoespaciada. xhtml2pdf soporta un subconjunto de CSS 2.1.
STYLESHEET = """
@page {
    size: a4 portrait;
    margin: 1.9cm 1.6cm 2.1cm 1.6cm;
    @frame footer { -pdf-frame-content: footer; bottom: 1.0cm; margin-left: 1.6cm;
                    margin-right: 1.6cm; height: 1cm; }
}
body { font-family: Helvetica, Arial, sans-serif; font-size: 8.6pt; line-height: 1.42;
       color: #1a1a1a; }
h1 { font-size: 17pt; color: #0d3b66; margin: 0 0 4pt 0; border-bottom: 1.6pt solid #1f6feb;
     padding-bottom: 4pt; }
h2 { font-size: 12pt; color: #0d3b66; margin: 15pt 0 5pt 0; border-bottom: 0.6pt solid #c9d6e8;
     padding-bottom: 2pt; }
h3 { font-size: 10pt; color: #1f4e79; margin: 11pt 0 4pt 0; }
h4 { font-size: 9pt; color: #1f4e79; margin: 9pt 0 3pt 0; }
p { margin: 0 0 5pt 0; text-align: left; }
ul, ol { margin: 0 0 6pt 14pt; padding: 0; }
li { margin: 0 0 2pt 0; }
a { color: #1f6feb; text-decoration: none; }
code { font-family: Courier, monospace; font-size: 8pt; background-color: #f2f4f8;
       color: #7a2048; }
pre { font-family: Courier, monospace; font-size: 7.4pt; background-color: #f6f8fa;
      border: 0.5pt solid #d8dee6; border-left: 2.4pt solid #1f6feb; padding: 5pt;
      margin: 5pt 0 7pt 0; line-height: 1.28; }
pre code { background-color: transparent; color: #24292f; }
blockquote { border-left: 2.4pt solid #d29922; background-color: #fffaf0; padding: 5pt 7pt;
             margin: 5pt 0 7pt 0; color: #4a3c14; }
blockquote p { margin: 0; }
table { border-collapse: collapse; width: 100%; margin: 5pt 0 8pt 0; font-size: 7.4pt; }
th { background-color: #0d3b66; color: #ffffff; border: 0.5pt solid #0d3b66; padding: 3.2pt 4pt;
     text-align: left; font-weight: bold; }
td { border: 0.5pt solid #c9d6e8; padding: 3.2pt 4pt; vertical-align: top;
     word-wrap: break-word; }
hr { border: none; border-top: 0.5pt solid #c9d6e8; margin: 10pt 0; }
.cover { text-align: center; padding-top: 3.0cm; }
.cover-system { font-size: 12pt; color: #1f6feb; letter-spacing: 1.2pt; margin-bottom: 6pt; }
.cover-title { font-size: 23pt; color: #0d3b66; font-weight: bold; margin: 10pt 3cm 16pt 3cm;
               line-height: 1.2; }
.cover-rule { border-top: 1.6pt solid #1f6feb; width: 42%; margin: 0 auto 16pt auto; }
.cover-meta { font-size: 9.5pt; color: #444444; line-height: 1.85; }
.cover-note { font-size: 7.6pt; color: #777777; margin-top: 1.6cm; }
.toc { margin-top: 10pt; }
.toc-title { font-size: 11pt; color: #0d3b66; font-weight: bold; border-bottom: 0.6pt solid #c9d6e8;
             padding-bottom: 3pt; }
.toc-item { margin: 0 0 3.5pt 12pt; font-size: 8.8pt; color: #24292f; }
.diagram { margin: 6pt 0 9pt 0; }
.diagram-label { font-size: 7.4pt; color: #6a737d; font-style: italic; margin: 0 0 2pt 0; }
.diagram-code { font-family: Courier, monospace; font-size: 6.9pt; background-color: #f6f8fa;
                border: 0.5pt dashed #a8b4c4; padding: 5pt; line-height: 1.25; }
#footer { font-size: 7pt; color: #8a8a8a; text-align: center; }
"""

PAGE_TEMPLATE = """<!DOCTYPE html>
<html>
<head><meta charset="utf-8" /><title>{title}</title>
<style>{stylesheet}</style></head>
<body>
<div id="footer">{system} &middot; {title} &middot; v{version} &middot; {date}
&middot; pag. <pdf:pagenumber /></div>
<div class="cover">
  <p class="cover-system">{system}</p>
  <p class="cover-title">{title}</p>
  <div class="cover-rule"></div>
  <p class="cover-meta">
    Documentacion del sistema<br />
    Version analizada: <b>{version}</b><br />
    Commit analizado: <b>{commit}</b><br />
    Fecha de generacion: <b>{date}</b>
  </p>
  <p class="cover-note">
    Generado a partir de {source} &mdash; los Markdown son la fuente unica.<br />
    No editar este PDF: editar el Markdown y regenerar.
  </p>
</div>
<pdf:nextpage />
{toc}
{content}
</body>
</html>
"""


# Longitud a partir de la cual un identificador dentro de una celda se parte.
# Con la fuente monoespaciada de las tablas (7,4 pt), una columna estrecha admite
# del orden de 16 caracteres antes de desbordar sobre la columna siguiente.
MAX_CODE_CHARS_IN_CELL = 16


def _split_long_token(token: str) -> str:
    """Parte un identificador largo en trozos unidos por `<br />`.

    Corta por los separadores naturales del identificador (``::``, ``_``, ``-``,
    ``/``, ``.``) y agrupa los trozos hasta el ancho máximo, de modo que
    ``InspectorService::collect_snapshot`` se imprime en dos líneas legibles en
    vez de derramarse sobre la celda contigua.
    """
    partes = re.split(r"(::|[_\-/.])", token)
    lineas: list[str] = [""]
    for parte in partes:
        if lineas[-1] and len(lineas[-1]) + len(parte) > MAX_CODE_CHARS_IN_CELL:
            lineas.append("")
        lineas[-1] += parte
    return "<br />".join(linea for linea in lineas if linea)


def allow_breaks_in_long_code(html_text: str) -> str:
    """Evita que un identificador largo desborde su celda en las tablas.

    Un símbolo como ``InspectorService::collect_snapshot`` es una "palabra"
    indivisible para el motor de PDF: sin partirla, se derrama sobre la columna
    siguiente y deja la tabla ilegible —un defecto que no se detecta mirando solo
    que el archivo exista—. La transformación se aplica **solo dentro de celdas**
    (`<td>` y `<th>`) y no toca el Markdown de origen, que sigue siendo la fuente
    única.
    """

    def partir_celda(celda: re.Match) -> str:
        def partir_codigo(codigo: re.Match) -> str:
            contenido = codigo.group(1)
            if len(contenido) <= MAX_CODE_CHARS_IN_CELL or "<" in contenido:
                return codigo.group(0)
            return f"<code>{_split_long_token(contenido)}</code>"

        return re.sub(
            r"<code>(.*?)</code>", partir_codigo, celda.group(0), flags=re.DOTALL
        )

    return re.sub(
        r"<t[dh][^>]*>.*?</t[dh]>", partir_celda, html_text, flags=re.DOTALL
    )


def render_document(
    md_module,
    pisa_module,
    md_path: Path,
    out_path: Path,
    version: str,
    commit: str,
    today: str,
) -> tuple[bool, int, str]:
    """Convierte un Markdown a PDF.

    Devuelve ``(exito, numero_de_diagramas, mensaje_de_error)``.
    """
    raw = md_path.read_text(encoding="utf-8")
    title = document_title(raw, md_path.stem)

    body_md, diagram_count = replace_mermaid_blocks(raw)
    # El H1 ya aparece en la portada; quitarlo del cuerpo evita duplicarlo.
    body_md = re.sub(r"^# .*?\n", "", body_md, count=1)
    toc_html = build_toc(body_md)

    content_html = md_module.markdown(
        body_md,
        extensions=["tables", "fenced_code", "sane_lists", "attr_list", "md_in_html"],
    )
    content_html = allow_breaks_in_long_code(content_html)

    document = PAGE_TEMPLATE.format(
        title=html.escape(title),
        system=html.escape(SYSTEM_NAME),
        version=html.escape(version),
        commit=html.escape(commit),
        date=html.escape(today),
        source=html.escape(md_path.name),
        stylesheet=STYLESHEET,
        toc=toc_html,
        content=content_html,
    )

    buffer = io.BytesIO()
    # `show_error_as_pdf=False`: preferimos fallar y avisar antes que generar un
    # PDF cuyo contenido sea el mensaje de error.
    status = pisa_module.CreatePDF(
        io.StringIO(document), dest=buffer, encoding="utf-8", show_error_as_pdf=False
    )
    if status.err:
        return False, diagram_count, f"xhtml2pdf devolvio {status.err} error(es)"

    out_path.write_bytes(buffer.getvalue())
    return True, diagram_count, ""


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Genera los PDF de docs/system-documentation/ a partir de los Markdown."
    )
    parser.add_argument(
        "filtros",
        nargs="*",
        help="Prefijos de archivo a generar (p. ej. 07 11). Sin filtros, genera todos.",
    )
    parser.add_argument(
        "--out", default=None, help="Directorio de salida alternativo."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Solo comprueba dependencias y archivos, sin generar nada.",
    )
    args = parser.parse_args()

    root = repo_root()
    docs_dir = root / DOCS_DIRNAME
    out_dir = Path(args.out) if args.out else docs_dir / PDF_DIRNAME

    if not docs_dir.is_dir():
        print(f"No existe el directorio de documentacion: {docs_dir}", file=sys.stderr)
        return 1

    # Los volúmenes exFAT/FAT (habituales en discos externos) siembran
    # bifurcaciones AppleDouble `._archivo.md` que no son texto UTF-8. Se
    # descartan igual que hace .markdownlint-cli2.jsonc.
    sources = sorted(p for p in docs_dir.glob("*.md") if not p.name.startswith("._"))
    if args.filtros:
        sources = [p for p in sources if any(p.name.startswith(f) for f in args.filtros)]

    if not sources:
        print("No hay documentos que generar con esos filtros.", file=sys.stderr)
        return 1

    md_module, pisa_module = _load_dependencies()
    if md_module is None:
        return 1

    version = read_version(root)
    commit = read_commit(root)
    today = _dt.date.today().isoformat()

    if args.check:
        print("Dependencias disponibles: markdown, xhtml2pdf")
        print(f"Documentos detectados: {len(sources)}")
        print(f"Version: {version}  -  Commit: {commit}")
        print(f"Salida: {out_dir}")
        return 0

    out_dir.mkdir(parents=True, exist_ok=True)

    generated, failed, total_diagrams = 0, [], 0
    print(f"Generando PDF - {SYSTEM_NAME} v{version} - commit {commit}")
    print(f"Salida: {out_dir}\n")

    for md_path in sources:
        out_path = out_dir / (md_path.stem + ".pdf")
        ok, diagrams, error = render_document(
            md_module, pisa_module, md_path, out_path, version, commit, today
        )
        total_diagrams += diagrams
        if ok:
            size_kb = out_path.stat().st_size / 1024
            extra = f"  [{diagrams} diagrama(s) como codigo]" if diagrams else ""
            print(f"  OK    {out_path.name:<42} {size_kb:7.1f} KB{extra}")
            generated += 1
        else:
            print(f"  FALLO {out_path.name:<42} {error}", file=sys.stderr)
            failed.append(md_path.name)

    print(f"\n{generated} PDF generado(s) en {out_dir}")
    if total_diagrams:
        print(
            f"{total_diagrams} diagrama(s) Mermaid incluidos como codigo fuente "
            "(el motor de PDF no ejecuta JavaScript)."
        )
    if failed:
        print(f"{len(failed)} fallo(s): {', '.join(failed)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
