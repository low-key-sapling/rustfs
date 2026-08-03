## —— Single Architecture Docker Builds (Traditional) ----------------------------------------------

.PHONY: docker-build-production
docker-build-production: ## Build single-arch production image
	@echo "🏗️ Building single-architecture production Docker image..."
	@echo "💡 Consider using 'make docker-buildx-production-local' for multi-arch support"
	$(DOCKER_CLI) build --build-arg RELEASE_REPOSITORY=$(RELEASE_REPOSITORY) -f $(DOCKERFILE_PRODUCTION) -t rustfs:latest .

.PHONY: docker-build-source
docker-build-source: ## Build single-arch source image
	@echo "🏗️ Building single-architecture source Docker image..."
	@echo "💡 Consider using 'make docker-dev-local' for multi-arch support"
	DOCKER_BUILDKIT=1 $(DOCKER_CLI) build \
		--build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(DOCKERFILE_SOURCE) -t rustfs:source .
