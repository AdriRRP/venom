import {
	createRootRoute,
	createRoute,
	createRouter,
} from "@tanstack/react-router";
import { FindingsPage } from "../routes/findings";

const rootRoute = createRootRoute();

const findingsRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/",
	component: FindingsPage,
});

const routeTree = rootRoute.addChildren([findingsRoute]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}
