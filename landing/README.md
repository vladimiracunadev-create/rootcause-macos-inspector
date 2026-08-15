# Landing

Página del producto, servida por GitHub Pages desde este mismo repositorio mediante el workflow
[`deploy-landing.yml`](../.github/workflows/deploy-landing.yml).

## Contenido

```text
landing/
├── index.html          ← la página, sin dependencias externas
└── assets/
    ├── style.css       ← estilos, con la paleta de la app
    └── favicon.svg     ← el mismo radar que dibuja la interfaz
```

## Reglas

- **Sin dependencias externas.** Nada de CDN, tipografías remotas ni analítica. La página no debe
  hacer una sola petición fuera de su propio origen — sería incoherente en el sitio de un producto
  que presume de análisis local.
- **La paleta es la de la app.** Los colores de `style.css` son los mismos de `src/app.rs`.
- **Nada que la app no haga.** Si la landing lo promete, el producto lo hace.

## Probar en local

```bash
python3 -m http.server 8080 --directory landing
open http://localhost:8080
```

## Configurar Pages

En el repositorio: **Settings → Pages → Source: GitHub Actions**. El workflow se encarga del resto
en cada cambio dentro de `landing/`.
