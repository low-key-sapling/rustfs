## —— Production builds using docker buildx (for CI/CD and production) -----------------------------

.PHONY: docker-buildx
docker-buildx: ## Build production multi-arch image (no push)
	@echo "🏗️ Building multi-architecture production Docker images with buildx..."
	./docker-buildx.sh

.PHONY: docker-buildx-push
docker-buildx-push: ## Build and push production multi-arch image
	@echo "🚀 Building and pushing multi-architecture production Docker images with buildx..."
	./docker-buildx.sh --push

.PHONY: docker-buildx-version
docker-buildx-version: ## Build and version production multi-arch image # e.g (make docker-buildx-version VERSION=v1.0.0)
	@if [ -z "$(VERSION)" ]; then \
		echo "❌ Error: Please specify version, example: make docker-buildx-version VERSION=v1.0.0"; \
		exit 1; \
	fi
	@echo "🏗️ Building multi-architecture production Docker images (version: $(VERSION))..."
	./docker-buildx.sh --release $(VERSION)

.PHONY: docker-buildx-push-version
docker-buildx-push-version: ## Build and version and push production multi-arch image # e.g (make docker-buildx-push-version VERSION=v1.0.0)
	@if [ -z "$(VERSION)" ]; then \
		echo "❌ Error: Please specify version, example: make docker-buildx-push-version VERSION=v1.0.0"; \
		exit 1; \
	fi
	@echo "🚀 Building and pushing multi-architecture production Docker images (version: $(VERSION))..."
	./docker-buildx.sh --release $(VERSION) --push

.PHONY: docker-buildx-production-local
docker-buildx-production-local: ## Build production single-arch image locally
	@echo "🏗️ Building single-architecture production Docker image locally..."
	@echo "💡 Alternative to docker-buildx.sh for local testing"
	$(DOCKER_CLI) buildx build \
		--file $(DOCKERFILE_PRODUCTION) \
		--tag rustfs:production-latest \
		--tag rustfs:latest \
		--load \
		--build-arg RELEASE=latest \
		--build-arg RELEASE_REPOSITORY=$(RELEASE_REPOSITORY) \
		.
