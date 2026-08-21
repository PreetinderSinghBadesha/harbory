import { Navigate, Outlet } from "react-router-dom";
import { useAuth } from "../context/AuthContext";

export function ProtectedRoute() {
  const { session, loading } = useAuth();

  if (loading) return <p className="page-status">Loading…</p>;
  if (!session) return <Navigate to="/login" replace />;

  return <Outlet />;
}
