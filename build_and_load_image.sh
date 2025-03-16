#!/bin/bash

# Validate input parameter for TAG
if [ "$#" -ne 1 ]; then
  # Set default value for TAG if not provided
  TAG=${1:-0.1}
else
  TAG=$1
fi



# Validate TAG format (should be in the form of 'X.Y')
if [[ ! $TAG =~ ^[0-9]+\.[0-9]+$ ]]; then
  echo "Error: Invalid tag format. Must be in the form of 'X.Y' where X and Y are integers."
  exit 1
fi

# Build the Docker image
echo "Building Docker image..."
podman build -q -t "docker.io/arnecdn/greeting-api:${TAG}" . || {
  echo "Error: Docker build failed."
  exit 1
}

# Create .docker directory and save the image
mkdir -p .docker
echo "Saving Docker image to .docker/greeting-api.tar..."
podman save -o .docker/greeting-api.tar "docker.io/arnecdn/greeting-api:${TAG}" || {
  echo "Error: Failed to save Docker image."
  exit 1
}

# Load the image into Minikube
echo "Loading image into Minikube..."
minikube image load .docker/greeting-api.tar || {
  echo "Error: Failed to load image into Minikube."
  exit 1
}

echo "Process completed successfully!"

# apply the kubernetes deployment for kubernetes/greeting-rust.yaml with TAG
kubectl apply -f  kubernetes/greeting_api.yaml --record

