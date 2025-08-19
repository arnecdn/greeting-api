TAG ?= 0.2
APP_NAME = greeting_api
IMAGE_NAME = arnecdn/$(APP_NAME)
KUBERNETES_FILE = kubernetes/$(APP_NAME).yaml

.PHONY: build_app all build_image deploy clean validate-tag

all: build_image deploy

build_app:
	@echo "Building the application..."
	cargo build --release || { \
		echo "Error: Cargo build failed."; \
		exit 1; \
	}

validate-tag:
	@if ! echo $(TAG) | grep -Eq '^[0-9]+\.[0-9]+$$'; then \
		echo "Error: Invalid tag format. Must be in the form of 'X.Y' where X and Y are integers."; \
		exit 1; \
	fi

build_image: validate-tag
	@echo "Building Docker image directly in Minikube..."
	minikube image build -t "$(IMAGE_NAME):$(TAG)" . || { \
		echo "Error: Docker build failed."; \
		exit 1; \
	}

deploy: build_image
	@echo "Applying Kubernetes deployment..."
	kubectl apply -f $(KUBERNETES_FILE) --record || { \
		echo "Error: Failed to apply Kubernetes deployment."; \
		exit 1; \
	}

clean:
	@echo "Cleaning up..."
	@echo "Removing image from Minikube..."
	minikube image rm "$(IMAGE_NAME):$(TAG)" 2>/dev/null || true