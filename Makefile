TAG ?= 0.1
APP_NAME = greeting_api
IMAGE_NAME = docker.io/arnecdn/$(APP_NAME)
DOCKER_DIR = .docker
DOCKER_FILE = $(DOCKER_DIR)/$(APP_NAME).tar
KUBERNETES_FILE = kubernetes/$(APP_NAME).yaml

.PHONY: build_app all build_image save_image load_image deploy clean

all: build_image save_image load_image deploy

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
	@echo "Building Docker image..."
	podman build -t "$(IMAGE_NAME):$(TAG)" . || { \
		echo "Error: Docker build failed."; \
		exit 1; \
	}

save_image: build_image
	@echo "Saving Docker image to $(DOCKER_FILE)..."
	mkdir -p $(DOCKER_DIR)
	podman save -o $(DOCKER_FILE) "$(IMAGE_NAME):$(TAG)" || { \
		echo "Error: Failed to save Docker image."; \
		exit 1; \
	}

load_image: save_image
	@echo "Loading image into Minikube..."
	minikube image load $(DOCKER_FILE) || { \
		echo "Error: Failed to load image into Minikube."; \
		exit 1; \
	}

deploy: load_image
	@echo "Applying Kubernetes deployment..."
	kubectl apply -f $(KUBERNETES_FILE) --record || { \
		echo "Error: Failed to apply Kubernetes deployment."; \
		exit 1; \
	}

clean:
	@echo "Cleaning up..."
	rm -rf $(DOCKER_DIR)