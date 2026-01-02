# Harbory

Harbory is a platform that enables users to deploy frontend and backend projects with a simple interface, providing project deployment on a private IP with subdomains (e.g., *project.harbory.domain.com*).

## Features

- **Advanced Docker Management**: Comprehensive control over your Docker ecosystem. Visualize, monitor, and manage containers, images, volumes, and networks through an intuitive interface.
- **Automated Reverse Proxy**: Seamless subdomain orchestration. Harbory automatically configures Nginx to route traffic to your deployments using concise private subdomains (e.g., `project.harbory.domain.com`), eliminating manual configuration.
- **Real-Time Analytics**: Deep insights into system health and project performance. Monitor server resources, track storage utilization, and analyze application speed to ensure optimal operation.
- **Streamlined Deployment**: Direct integration with GitHub. Deploy your frontend and backend repositories with ease, checking out code and launching services in just a few clicks.

## Development Setup

Follow these instructions to set up the development environment for both the backend and frontend.

### Prerequisites

- [Go](https://go.dev/dl/) (for the backend)
- [Node.js](https://nodejs.org/en/download/) & npm (for the frontend)
- [Docker](https://docs.docker.com/get-docker/) (running on the system)

### Backend (Go)

The backend is located in the `harbory-backend` directory.

1. Navigate to the backend directory:
   ```bash
   cd harbory-backend
   ```

2. Download dependencies:
   ```bash
   go mod download
   ```

3. Run the server:
   ```bash
   go run cmd/server/main.go -config="<path-to-config-file>"
   ```

### Frontend (SvelteKit)

The frontend is located in the `harbory-frontend` directory.

1. Navigate to the frontend directory:
   ```bash
   cd harbory-frontend
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Start the development server:
   ```bash
   npm run dev
   ```

   The application should now be accessible at `http://localhost:5173` (or the port shown in the terminal).
