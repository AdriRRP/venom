import {
	createRootRoute,
	createRoute,
	createRouter,
	redirect,
} from "@tanstack/react-router";
import { FindingsPage } from "../routes/findings";
import { OperationsPage } from "../routes/operations";

const rootRoute = createRootRoute({
	beforeLoad: async ({ location }) => {
		if (location.pathname === "/") {
			throw redirect({ to: "/findings" });
		}
	},
});

const findingsRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/findings",
	component: FindingsPage,
});

const operationsRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/operations",
	component: OperationsPage,
});

const routeTree = rootRoute.addChildren([findingsRoute, operationsRoute]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}
