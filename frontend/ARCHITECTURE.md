# Frontend Architecture

The frontend is a Next.js application intended to provide authenticated users with a responsive interface for portfolio viewing, asset discovery, basic charting, and activity flows. At present, it operates as a UI prototype with minimal data plumbing.

## Stack

- Next.js 13.4.19 - React framework with App Router
- React 18 - Component library with concurrent features
- TypeScript 5.2 - Static type checking
- Turbopack - Development bundler
- Clerk 6.18.0 - Third-party authentication service with middleware-based route protection enforced at the edge
- TailwindCSS v3.3.2 - Utility-first CSS framework
- shadcn/ui - Component library built on Radix primitives
- Lucide React - Icon system
- Recharts 2.15.3 - Charting library
- PostgreSQL - Connection pool via `pg` library (scaffolded but unused)

The app uses the Next.js App Router for routing. Routes are grouped, like `(consumer)`, to handle logged-in user areas. Middleware checks if users are logged in before letting them access secure pages. The design splits server and client parts, making the app fast and easy to grow.

The component model is organized into three layers. The first is the **Primitive Layer**, which includes Radix-based UI components stored in the `ui/` folder. These components are simple, accessible, and support themes. The second is the **Feature Layer**, which combines these primitives into business-specific components like the sidebar, mini-portfolio, and watchlist. The third is the **Page Layer**, where route-specific components handle tasks like fetching data, managing suspense, and dealing with errors.

### Styling System
- TailwindCSS v4 with CSS layers (`theme`, `base`, `components`, `utilities`)
- CSS custom properties for theming with OKLCH color space
- Light/dark mode support via class-based theming
- Responsive sidebar behavior with full, compact, and hidden states
