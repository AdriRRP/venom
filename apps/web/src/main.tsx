import React from "react";
import ReactDOM from "react-dom/client";
import { AppShell } from "./app/app-shell";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
	throw new Error("missing root element");
}

ReactDOM.createRoot(root).render(
	<React.StrictMode>
		<AppShell statusLabel="Pending wiring">
			<section className="panel">
				<div className="panel-header">
					<div>
						<p className="eyebrow">Bootstrap</p>
						<h2>UI Shell Ready</h2>
					</div>
				</div>
				<p className="copy">
					The first VENOM operator console scaffold is in place and ready for
					API wiring.
				</p>
			</section>
		</AppShell>
	</React.StrictMode>,
);
